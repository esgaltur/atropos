use axum::{
    routing::{get, post, delete, patch},
    Router,
};
use std::sync::Arc;
use crate::application::{
    allocation_service::AllocationService,
    pool_service::PoolService,
    resource_service::ResourceService,
};
use crate::infrastructure::postgres_repository::PostgresRepository;

use super::{handlers, ui};

#[derive(Clone)]
pub struct AppState {
    pub pool_service: Arc<PoolService<PostgresRepository>>,
    pub resource_service: Arc<ResourceService<PostgresRepository>>,
    pub allocation_service: Arc<AllocationService<PostgresRepository>>,
}

pub fn create_router(state: AppState) -> Router {
    Router::new()
        .route("/", get(ui::admin_dashboard))
        .route("/admin", get(ui::admin_dashboard))
        .route("/admin/audit-log", get(ui::audit_log_stream))
        .route("/health", get(handlers::health_check))
        .route("/pools", post(handlers::create_pool))
        .route("/resources", post(handlers::register_resource))
        .route("/leases", post(handlers::allocate_lease))
        .route("/leases/:id", delete(handlers::release_lease))
        .route("/leases/:id/renew", patch(handlers::renew_lease))
        .with_state(state)
}
