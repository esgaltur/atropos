use sqlx::{PgPool, Row};
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
        let mut prune_interval = time::interval(Duration::from_secs(3600));
        let mut health_interval = time::interval(Duration::from_secs(60));
        loop {
            tokio::select! {
                _ = prune_interval.tick() => {
                    self.prune_database().await;
                }
                _ = health_interval.tick() => {
                    self.check_resource_health().await;
                }
            }
        }
    }

    async fn check_resource_health(&self) {
        // Find a batch of Healthy resources to check
        let rows =
            sqlx::query("SELECT id, external_id FROM resources WHERE status = 'Healthy' LIMIT 100")
                .fetch_all(&self.pool)
                .await;

        if let Ok(resources) = rows {
            for row in resources {
                let id: uuid::Uuid = row.get("id");
                let external_id: String = row.get("external_id");

                // Simulate checking external API (e.g. K8s Node status)
                let is_healthy = self.ping_external_system(&external_id).await;

                if !is_healthy {
                    info!(
                        "Resource {} ({}) failed health check. Moving to Draining.",
                        id, external_id
                    );
                    sqlx::query("UPDATE resources SET status = 'Draining' WHERE id = $1")
                        .bind(id)
                        .execute(&self.pool)
                        .await
                        .ok();

                    sqlx::query("INSERT INTO audit_log (action, resource_id, details) VALUES ('AUTO_DRAIN', $1, $2)")
                        .bind(id)
                        .bind(serde_json::json!({ "reason": "health_check_failed" }))
                        .execute(&self.pool)
                        .await
                        .ok();
                }
            }
        }
    }

    async fn ping_external_system(&self, _external_id: &str) -> bool {
        // Simulated: Use system time to fail randomly (~1% chance)
        if let Ok(duration) = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
            duration.subsec_nanos() % 100 != 0
        } else {
            true // Default to healthy if time fails
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
