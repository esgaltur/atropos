use std::time::Duration;
use tokio::time;
use tracing::{info, error};
use sqlx::PgPool;

pub struct ReaperService {
    pool: PgPool,
    batch_size: i64,
}

impl ReaperService {
    pub fn new(pool: PgPool) -> Self {
        Self { 
            pool,
            batch_size: 1000, // Default batch size
        }
    }

    pub fn with_batch_size(mut self, size: i64) -> Self {
        self.batch_size = size;
        self
    }

    pub async fn run(&self) {
        let mut interval = time::interval(Duration::from_secs(10));
        loop {
            interval.tick().await;
            loop {
                let count = self.reclaim_expired().await;
                if count < self.batch_size {
                    break;
                }
            }
        }
    }

    pub async fn reclaim_expired(&self) -> i64 {
        // Optimized batched reclamation using SKIP LOCKED for high concurrency.
        let res = sqlx::query(
            r#"
            WITH target_leases AS (
                SELECT id 
                FROM leases 
                WHERE status = 'ACTIVE' AND expires_at <= NOW()
                LIMIT $1
                FOR UPDATE SKIP LOCKED
            )
            UPDATE leases
            SET status = 'EXPIRED'
            FROM target_leases
            WHERE leases.id = target_leases.id
            "#
        )
        .bind(self.batch_size)
        .execute(&self.pool)
        .await;

        match res {
            Ok(result) => {
                let count = result.rows_affected() as i64;
                if count > 0 {
                    info!("Reaper reclaimed {} expired leases in a batch.", count);
                    metrics::counter!("reclaim_success_total").increment(count as u64);
                }
                count
            }
            Err(e) => {
                error!("Reaper failed to execute batched reclamation: {:?}", e);
                metrics::counter!("reclaim_failure_total").increment(1);
                0
            }
        }
    }
}
