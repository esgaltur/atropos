use std::sync::Arc;
use sqlx::postgres::PgPoolOptions;
use tokio::net::TcpListener;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use metrics_exporter_prometheus::PrometheusBuilder;
use tokio::signal;

use atropos::api::routes::{create_router, AppState};
use atropos::application::{
    allocation_service::AllocationService,
    pool_service::PoolService,
    resource_service::ResourceService,
    reaper::ReaperService,
    maintenance::MaintenanceService,
};
use atropos::infrastructure::postgres_repository::PostgresRepository;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. LOGGING & OBSERVABILITY
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("Atropos Resource Allocator starting up...");

    // 2. METRICS (Prometheus)
    PrometheusBuilder::new()
        .with_http_listener(([0, 0, 0, 0], 9000))
        .install()
        .expect("Failed to install Prometheus recorder");
    tracing::info!("Prometheus metrics available on 0.0.0.0:9000/metrics");

    // 3. DATABASE
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    tracing::info!("Connecting to database...");
    let pool = PgPoolOptions::new()
        .max_connections(100) // Tuned for higher concurrency
        .connect(&database_url)
        .await?;

    // 4. DEPENDENCY INJECTION (Composition Root)
    let repo = Arc::new(PostgresRepository::new(pool.clone()));
    let pool_service = Arc::new(PoolService::new(repo.clone()));
    let resource_service = Arc::new(ResourceService::new(repo.clone()));
    let allocation_service = Arc::new(AllocationService::new(repo.clone()));

    // 5. BACKGROUND REAPER with Cancellation
    let reaper = ReaperService::new(pool.clone(), repo.clone());
    let reaper_handle = tokio::spawn(async move {
        tracing::info!("Starting background Reaper Service...");
        reaper.run().await;
    });

    // 5b. BACKGROUND MAINTENANCE
    let maintenance = MaintenanceService::new(pool.clone());
    let maintenance_handle = tokio::spawn(async move {
        tracing::info!("Starting background Maintenance Service...");
        maintenance.run().await;
    });

    let app_state = AppState {
        pool_service,
        resource_service,
        allocation_service: allocation_service.clone(),
    };

    let app = create_router(app_state);

    // 6. gRPC SERVER (High Performance)
    let grpc_service = atropos::api::grpc::GrpcAllocationService::new(allocation_service.clone());
    let grpc_addr = "0.0.0.0:50051".parse()?;
    tracing::info!("gRPC Listening on {}", grpc_addr);

    let grpc_server = tonic::transport::Server::builder()
        .add_service(atropos::api::grpc::atropos_v1::allocation_service_server::AllocationServiceServer::new(grpc_service))
        .serve(grpc_addr);

    // 7. GRACEFUL SHUTDOWN & DUAL SERVER RUN
    let listener = TcpListener::bind("0.0.0.0:3000").await?;
    tracing::info!("API Listening on {}", listener.local_addr()?);
    
    let axum_server = axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal());

    tokio::select! {
        _ = axum_server => { tracing::info!("Axum server exited"); },
        _ = grpc_server => { tracing::info!("gRPC server exited"); },
    }

    tracing::info!("Shutting down. Waiting for background tasks...");
    reaper_handle.abort();
    maintenance_handle.abort();

    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => { tracing::info!("SIGINT received, starting graceful shutdown..."); },
        _ = terminate => { tracing::info!("SIGTERM received, starting graceful shutdown..."); },
    }
}
