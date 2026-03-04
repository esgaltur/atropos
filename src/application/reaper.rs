use std::time::Duration;
use tokio::time;
use tracing::{info, error};
use sqlx::PgPool;

pub struct ReaperService {
    pool: PgPool,
}

impl ReaperService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn run(&self) {
        let mut interval = time::interval(Duration::from_secs(10));
        loop {
            interval.tick().await;
            self.reclaim_expired().await;
        }
    }

    async fn reclaim_expired(&self) {
        let res = sqlx::query(
            r#"
            UPDATE leases
            SET status = 'EXPIRED'
            WHERE status = 'ACTIVE' AND expires_at <= NOW()
            "#
        )
        .execute(&self.pool)
        .await;

        match res {
            Ok(result) => {
                let count = result.rows_affected();
                if count > 0 {
                    info!("Reaper reclaimed {} expired leases.", count);
                    // Also emit metrics here if possible
                    metrics::counter!("reclaim_count").increment(count);
                }
            }
            Err(e) => {
                error!("Reaper failed to execute query: {:?}", e);
            }
        }
    }
}
