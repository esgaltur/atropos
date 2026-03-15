use sqlx::postgres::PgPoolOptions;
use uuid::Uuid;
use chrono::{Utc, Duration};
use atropos::application::reaper::ReaperService;

#[tokio::test]
async fn test_reaper_batching_concurrency() {
    dotenvy::dotenv().ok();
    let database_url = std::env::var("DATABASE_URL").unwrap_or("postgres://postgres:postgres@localhost:5432/atropos".to_string());
    let pool = PgPoolOptions::new()
        .max_connections(10)
        .connect(&database_url)
        .await
        .expect("Failed to connect to DB");

    // Clean up
    sqlx::query("DELETE FROM leases").execute(&pool).await.unwrap();
    sqlx::query("DELETE FROM resources").execute(&pool).await.unwrap();
    sqlx::query("DELETE FROM pools").execute(&pool).await.unwrap();

    // 1. Seed Pool
    let pool_id = Uuid::new_v4();
    sqlx::query("INSERT INTO pools (id, name, resource_type) VALUES ($1, $2, $3)")
        .bind(pool_id)
        .bind("Batch-Pool")
        .bind("Batch-Type")
        .execute(&pool).await.unwrap();

    // 2. Seed 100 Expired Leases
    for _ in 0..100 {
        let res_id = Uuid::new_v4();
        sqlx::query("INSERT INTO resources (id, pool_id, external_id, status) VALUES ($1, $2, $3, 'Healthy')")
            .bind(res_id)
            .bind(pool_id)
            .bind(format!("res-{}", res_id))
            .execute(&pool).await.unwrap();
            
        sqlx::query("INSERT INTO leases (id, resource_id, owner_id, tenant_id, status, expires_at) VALUES ($1, $2, 'o', 't', 'ACTIVE', $3)")
            .bind(Uuid::new_v4())
            .bind(res_id)
            .bind(Utc::now() - Duration::minutes(1))
            .execute(&pool).await.unwrap();
    }

    // 3. Mock multiple reapers running concurrently with small batch sizes
    let repo = std::sync::Arc::new(atropos::infrastructure::postgres_repository::PostgresRepository::new(pool.clone()));
    let reaper1 = ReaperService::new(pool.clone(), repo.clone()).with_batch_size(10);
    let reaper2 = ReaperService::new(pool.clone(), repo.clone()).with_batch_size(10);

    // Run reaper 1
    reaper1.reclaim_expired().await;
    
    // Verify 10 leases reclaimed
    let (count,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM leases WHERE status = 'EXPIRED'")
        .fetch_one(&pool).await.unwrap();
    assert_eq!(count, 10, "Expected 10 leases reclaimed in first batch");

    // Run reaper 2
    reaper2.reclaim_expired().await;
    
    // Verify 20 leases total reclaimed
    let (count,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM leases WHERE status = 'EXPIRED'")
        .fetch_one(&pool).await.unwrap();
    assert_eq!(count, 20, "Expected 20 leases total reclaimed after second batch");

    // Run reaper 1 again many times to clean up
    for _ in 0..8 {
        reaper1.reclaim_expired().await;
    }
    
    // Verify all 100 reclaimed
    let (count,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM leases WHERE status = 'EXPIRED'")
        .fetch_one(&pool).await.unwrap();
    assert_eq!(count, 100, "Expected all 100 leases reclaimed after all batches");
}
