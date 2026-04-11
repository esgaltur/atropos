use atropos::application::allocation_service::AllocationService;
use atropos::infrastructure::postgres_repository::PostgresRepository;
use sqlx::postgres::PgPoolOptions;
use std::sync::Arc;
use uuid::Uuid;

#[tokio::test]
async fn test_priority_preemption_logic() {
    dotenvy::dotenv().ok();
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or("postgres://postgres:postgres@localhost:5432/atropos".to_string());
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .expect("Failed to connect to DB");

    // Clean up
    sqlx::query("DELETE FROM audit_log").execute(&pool).await.unwrap();
    sqlx::query("DELETE FROM waitlist_entries").execute(&pool).await.unwrap();
    sqlx::query("DELETE FROM leases").execute(&pool).await.unwrap();
    sqlx::query("DELETE FROM resources").execute(&pool).await.unwrap();
    sqlx::query("DELETE FROM pools").execute(&pool).await.unwrap();

    let repo = Arc::new(PostgresRepository::new(pool.clone()));
    let service = AllocationService::new(repo.clone());
    let pool_type = "Preempt-Pool-Type".to_string();

    // 1. Setup: 1 Pool, 1 Resource
    let pool_id = Uuid::new_v4();
    sqlx::query("INSERT INTO pools (id, name, resource_type, policy) VALUES ($1, $2, $3, 'FIFO')")
        .bind(pool_id)
        .bind("Preempt-Pool")
        .bind(&pool_type)
        .execute(&pool).await.unwrap();

    let res_id = Uuid::new_v4();
    sqlx::query("INSERT INTO resources (id, pool_id, external_id, status) VALUES ($1, $2, $3, 'Healthy')")
        .bind(res_id)
        .bind(pool_id)
        .bind("preempt-res-1")
        .execute(&pool).await.unwrap();

    // 2. Step: Allocate with Priority 10 (Low)
    let lease_low = service.allocate(
        pool_type.clone(), 
        "low-user".into(), 
        "t1".into(), 
        10, 
        600, 
        None, 
        None,
        None,
        Some(false), 
        Some(false)
    ).await.expect("Low priority allocation should succeed");

    assert_eq!(lease_low.owner_id, "low-user");
    assert_eq!(lease_low.priority, 10);

    // 3. Step: Allocate with Priority 100 (High) AND preempt = true
    // This should evict low-user
    let lease_high = service.allocate(
        pool_type.clone(), 
        "high-user".into(), 
        "t1".into(), 
        100, 
        600, 
        None, 
        None,
        None,
        Some(false), 
        Some(true)
    ).await.expect("High priority allocation with preemption should succeed");

    assert_eq!(lease_high.owner_id, "high-user");
    assert_eq!(lease_high.priority, 100);
    assert_eq!(lease_high.resource_id, lease_low.resource_id);

    // 4. Assert: Old lease is REVOKED
    let (old_status,): (String,) = sqlx::query_as("SELECT status FROM leases WHERE id = $1")
        .bind(lease_low.id.0)
        .fetch_one(&pool).await.unwrap();
    assert_eq!(old_status, "REVOKED");

    // 5. Step: Try to allocate with Priority 50 (Medium) AND preempt = true
    // This should FAIL because high-user (100) is currently holding it.
    let result_mid = service.allocate(
        pool_type.clone(), 
        "mid-user".into(), 
        "t1".into(), 
        50, 
        600, 
        None, 
        None,
        None,
        Some(false), 
        Some(true)
    ).await;

    assert!(result_mid.is_err(), "Medium priority should NOT be able to preempt High priority");

    // 6. Assert Audit Logs
    let logs: Vec<(String,)> = sqlx::query_as("SELECT action FROM audit_log ORDER BY created_at ASC")
        .fetch_all(&pool).await.unwrap();
    
    // Expect: ALLOCATE (low), PREEMPT_REVOKE (low), PREEMPT_ALLOCATE (high)
    let actions: Vec<String> = logs.into_iter().map(|l| l.0).collect();
    assert!(actions.contains(&"ALLOCATE".to_string()));
    assert!(actions.contains(&"PREEMPT_REVOKE".to_string()));
    assert!(actions.contains(&"PREEMPT_ALLOCATE".to_string()));
}
