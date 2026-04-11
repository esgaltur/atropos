use atropos::application::reaper::ReaperService;
use chrono::{Duration, Utc};
use sqlx::postgres::PgPoolOptions;
use std::time::Instant;
use uuid::Uuid;

#[tokio::test]
async fn test_reaper_performance_baseline() {
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

    // 1. Seed Pool
    let pool_id = Uuid::new_v4();
    sqlx::query("INSERT INTO pools (id, name, resource_type) VALUES ($1, $2, $3)")
        .bind(pool_id)
        .bind("Baseline-Pool")
        .bind("Benchmark-Type")
        .execute(&pool)
        .await
        .unwrap();

    println!("Seeding 10,000 resources and leases...");
    let start_seed = Instant::now();

    let mut resources = Vec::new();
    for _ in 0..10000 {
        let res_id = Uuid::new_v4();
        resources.push(res_id);
    }

    // sqlx batch insert
    for chunk in resources.chunks(100) {
        let mut qb =
            sqlx::QueryBuilder::new("INSERT INTO resources (id, pool_id, external_id, status) ");
        qb.push_values(chunk, |mut b, &res_id| {
            b.push_bind(res_id)
                .push_bind(pool_id)
                .push_bind(format!("res-{}", res_id))
                .push_bind("Healthy");
        });
        let query = qb.build();
        query.execute(&pool).await.unwrap();
    }

    // Batch insert for leases (expired)
    for chunk in resources.chunks(100) {
        let mut qb = sqlx::QueryBuilder::new(
            "INSERT INTO leases (id, resource_id, owner_id, tenant_id, priority, status, expires_at) ",
        );
        qb.push_values(chunk, |mut b, &res_id| {
            b.push_bind(Uuid::new_v4())
                .push_bind(res_id)
                .push_bind("bench-owner")
                .push_bind("bench-tenant")
                .push_bind(0)
                .push_bind("ACTIVE")
                .push_bind(Utc::now() - Duration::minutes(10));
        });
        let query = qb.build();
        query.execute(&pool).await.unwrap();
    }

    println!("Seeding completed in {:?}", start_seed.elapsed());

    // 2. Measure Reaper
    let reaper = ReaperService::new(
        pool.clone(),
        std::sync::Arc::new(
            atropos::infrastructure::postgres_repository::PostgresRepository::new(pool.clone()),
        ),
    )
    .with_batch_size(20000);
    println!("Running Reaper reclamation...");

    // Explain the query in a transaction to not affect the actual run
    let mut tx = pool.begin().await.unwrap();
    let explain_rows: Vec<(String,)> = sqlx::query_as(
        r#"
        EXPLAIN ANALYZE
        UPDATE leases
        SET status = 'EXPIRED'
        WHERE status = 'ACTIVE' AND expires_at <= NOW()
        "#,
    )
    .fetch_all(&mut *tx)
    .await
    .unwrap();
    tx.rollback().await.unwrap();

    for row in explain_rows {
        println!("{}", row.0);
    }

    let start_reclaim = Instant::now();
    let count = reaper.reclaim_expired().await;

    let duration = start_reclaim.elapsed();

    // Verify results
    let (expired_count,): (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM leases WHERE status = 'EXPIRED'")
            .fetch_one(&pool)
            .await
            .unwrap();

    println!("Reclaimed {} leases in {:?}", count, duration);

    assert!(expired_count >= 10000);
}
