use atropos::application::allocation_service::AllocationService;
use atropos::domain::LeaseId;
use atropos::infrastructure::postgres_repository::PostgresRepository;
use chrono::{Duration, Utc};
use sqlx::postgres::PgPoolOptions;
use std::sync::Arc;
use uuid::Uuid;

#[tokio::test]
async fn test_manual_release_waitlist_fulfillment() {
    dotenvy::dotenv().ok();
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or("postgres://postgres:postgres@localhost:5432/atropos".to_string());
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .expect("Failed to connect to DB");

    // Clean up
    sqlx::query("DELETE FROM audit_log")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM waitlist_entries")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM leases")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM resources")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM pools")
        .execute(&pool)
        .await
        .unwrap();

    let repo = Arc::new(PostgresRepository::new(pool.clone()));
    let service = AllocationService::new(repo.clone());
    let pool_type = "Release-Fulfill-Type".to_string();

    // 1. Setup: 1 Pool, 1 Resource, 1 Active Lease, 1 Waitlist Entry
    let pool_id = Uuid::new_v4();
    sqlx::query("INSERT INTO pools (id, name, resource_type, policy) VALUES ($1, $2, $3, 'FIFO')")
        .bind(pool_id)
        .bind("Release-Pool")
        .bind(&pool_type)
        .execute(&pool)
        .await
        .unwrap();

    let res_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO resources (id, pool_id, external_id, status) VALUES ($1, $2, $3, 'Healthy')",
    )
    .bind(res_id)
    .bind(pool_id)
    .bind("release-res-1")
    .execute(&pool)
    .await
    .unwrap();

    // Create an ACTIVE lease
    let lease_id = Uuid::new_v4();
    sqlx::query("INSERT INTO leases (id, resource_id, owner_id, tenant_id, status, expires_at) VALUES ($1, $2, 'current-user', 't1', 'ACTIVE', $3)")
        .bind(lease_id)
        .bind(res_id)
        .bind(Utc::now() + Duration::hours(1))
        .execute(&pool).await.unwrap();

    // Add to waitlist
    sqlx::query("INSERT INTO waitlist_entries (id, pool_type, owner_id, tenant_id, priority) VALUES ($1, $2, 'waiting-user', 't1', 10)")
        .bind(Uuid::new_v4())
        .bind(&pool_type)
        .execute(&pool).await.unwrap();

    // 2. Act: Release the lease
    service.release(LeaseId(lease_id)).await.unwrap();

    // 3. Assert:
    // Waitlist entry should be fulfilled.
    let (wait_count,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM waitlist_entries")
        .fetch_one(&pool)
        .await
        .unwrap();

    assert_eq!(
        wait_count, 0,
        "Waitlist entry should have been fulfilled after manual release"
    );

    let (active_lease_owner,): (String,) =
        sqlx::query_as("SELECT owner_id FROM leases WHERE status = 'ACTIVE'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(
        active_lease_owner, "waiting-user",
        "The waiting user should now have the active lease"
    );
}
