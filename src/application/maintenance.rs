use sqlx::PgPool;
use std::time::Duration;
use tokio::time;
use tracing::{error, info};

pub struct MaintenanceService {
    pool: PgPool,
}

impl MaintenanceService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn run(&self) {
        // Run maintenance every hour (in production this might be every 24h)
        let mut interval = time::interval(Duration::from_secs(3600));
        loop {
            interval.tick().await;
            self.prune_database().await;
        }
    }

    async fn prune_database(&self) {
        info!("Starting routine database maintenance and pruning...");

        // 1. Prune Old Leases (Keep 30 days of history for RELEASED/EXPIRED)
        let prune_leases = sqlx::query(
            r#"
            DELETE FROM leases 
            WHERE status IN ('RELEASED', 'EXPIRED', 'REVOKED') 
            AND expires_at < NOW() - INTERVAL '30 days'
            "#,
        )
        .execute(&self.pool)
        .await;

        match prune_leases {
            Ok(res) => info!("Maintenance: Pruned {} old leases.", res.rows_affected()),
            Err(e) => error!("Maintenance failed to prune leases: {}", e),
        }

        // 2. Prune Old Audit Logs (Keep 90 days of history)
        let prune_audit = sqlx::query(
            r#"
            DELETE FROM audit_log 
            WHERE created_at < NOW() - INTERVAL '90 days'
            "#,
        )
        .execute(&self.pool)
        .await;

        match prune_audit {
            Ok(res) => info!(
                "Maintenance: Pruned {} old audit logs.",
                res.rows_affected()
            ),
            Err(e) => error!("Maintenance failed to prune audit logs: {}", e),
        }

        // 3. Optional: VACUUM (Requires Postgres superuser or table owner permissions)
        // Note: Postgres autovacuum usually handles this, but we can trigger it for extreme loads
        // sqlx::query("VACUUM (ANALYZE) leases").execute(&self.pool).await.ok();
    }
}
