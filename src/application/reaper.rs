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
        // I set this to 10 seconds for testing, but in production we might 
        // want to make this configurable via environment variables.
        let mut interval = time::interval(Duration::from_secs(10));
        loop {
            interval.tick().await;
            self.reclaim_expired().await;
        }
    }

    pub async fn reclaim_expired(&self) {
        // TODO(esgaltur): Right now this just marks them as EXPIRED. 
        // I want to add a step that checks the waitlist and immediately 
        // re-allocates the resource to the next person in line.
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
