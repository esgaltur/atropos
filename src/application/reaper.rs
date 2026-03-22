use crate::domain::repository::AllocationRepository;
use sqlx::{PgPool, Row};
use std::sync::Arc;
use std::time::Duration;
use tokio::time;
use tracing::{error, info};

pub struct ReaperService<R: AllocationRepository> {
    pool: PgPool, // Keep pool for the specific reaper queries
    repo: Arc<R>,
    batch_size: i64,
}

impl<R: AllocationRepository> ReaperService<R> {
    pub fn new(pool: PgPool, repo: Arc<R>) -> Self {
        Self {
            pool,
            repo,
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
        // We join with pools to get the resource_type so we know which waitlists to fulfill.
        let rows = sqlx::query(
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
            FROM target_leases, resources r, pools p
            WHERE leases.id = target_leases.id
              AND leases.resource_id = r.id
              AND r.pool_id = p.id
            RETURNING p.resource_type
            "#,
        )
        .bind(self.batch_size)
        .fetch_all(&self.pool)
        .await;

        match rows {
            Ok(rows) => {
                let count = rows.len() as i64;
                if count > 0 {
                    info!("Reaper reclaimed {} expired leases in a batch.", count);
                    metrics::counter!("reclaim_success_total").increment(count as u64);

                    // Collect unique pool types to fulfill waitlist
                    let mut pool_types: std::collections::HashSet<String> = rows
                        .iter()
                        .map(|r| r.get::<String, _>("resource_type"))
                        .collect();

                    for pool_type in pool_types.drain() {
                        // Fulfill as many waitlist entries as possible for this type
                        // (until no more entries or no more resources)
                        while let Ok(Some(lease)) = self
                            .repo
                            .fulfill_next_waitlist_entry(pool_type.clone())
                            .await
                        {
                            info!(
                                "Automatically fulfilled waitlist entry for pool {} (Lease: {})",
                                pool_type, lease.id
                            );
                        }
                    }
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
