use atropos::domain::{repository::AllocationRepository, LeaseStatus};
use atropos::infrastructure::postgres_repository::PostgresRepository;
use sqlx::postgres::PgPoolOptions;
use uuid::Uuid;

#[tokio::test]
async fn test_waitlist_fulfillment_logic() {
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

    let repo = PostgresRepository::new(pool.clone());
    let pool_type = "Fulfillment-Type".to_string();

    // 1. Setup: 1 Pool, 1 Resource, 1 Waitlist Entry
    let pool_id = Uuid::new_v4();
    sqlx::query("INSERT INTO pools (id, name, resource_type, policy) VALUES ($1, $2, $3, 'FIFO')")
        .bind(pool_id)
        .bind("Fulfillment-Pool")
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
    .bind("res-1")
    .execute(&pool)
    .await
    .unwrap();

    repo.waitlist_resource(pool_type.clone(), "waiting-user".into(), "t1".into(), 10)
        .await
        .unwrap();

    // Verify 1 waitlist entry exists
    let (wait_count,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM waitlist_entries")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(wait_count, 1);

    // 2. Act: Fulfill waitlist
    let lease_opt = repo
        .fulfill_next_waitlist_entry(pool_type.clone())
        .await
        .unwrap();

    // 3. Assert: Lease should be created, waitlist entry should be gone
    assert!(
        lease_opt.is_some(),
        "Expected a lease to be created for the waitlisted user"
    );
    let lease = lease_opt.unwrap();
    assert_eq!(lease.owner_id, "waiting-user");
    assert_eq!(lease.status, LeaseStatus::Active);

    let (wait_count_after,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM waitlist_entries")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(
        wait_count_after, 0,
        "Waitlist entry should be removed after fulfillment"
    );

    let (lease_count,): (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM leases WHERE status = 'ACTIVE'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(lease_count, 1, "There should be 1 active lease now");

    // 4. Edge Case: No waitlist entry
    let res = repo
        .fulfill_next_waitlist_entry("Non-Existent-Type".to_string())
        .await
        .unwrap();
    assert!(res.is_none());

    // 5. Edge Case: No resource available
    sqlx::query("INSERT INTO waitlist_entries (id, pool_type, owner_id, tenant_id, priority) VALUES ($1, $2, 'o2', 't1', 5)")
        .bind(Uuid::new_v4())
        .bind(&pool_type)
        .execute(&pool).await.unwrap();

    // Fill the resource (it's already leased in step 3)
    let res = repo
        .fulfill_next_waitlist_entry(pool_type.clone())
        .await
        .unwrap();
    assert!(res.is_none(), "Should not fulfill if no resource available");
}
