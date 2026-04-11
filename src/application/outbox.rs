use sqlx::{PgPool, Row};
use std::time::Duration;
use tokio::time;
use tracing::{error, info};
use uuid::Uuid;

pub struct OutboxService {
    pool: PgPool,
}

impl OutboxService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn run(&self) {
        let mut interval = time::interval(Duration::from_secs(5));
        loop {
            interval.tick().await;
            self.process_events().await;
        }
    }

    async fn process_events(&self) {
        // Use SKIP LOCKED to safely consume events even with multiple outbox worker nodes
        let rows = sqlx::query(
            r#"
            WITH target_events AS (
                SELECT id 
                FROM outbox_events 
                WHERE status = 'PENDING'
                ORDER BY created_at ASC
                LIMIT 50
                FOR UPDATE SKIP LOCKED
            )
            UPDATE outbox_events
            SET status = 'PROCESSING'
            FROM target_events
            WHERE outbox_events.id = target_events.id
            RETURNING outbox_events.id, outbox_events.event_type, outbox_events.payload
            "#,
        )
        .fetch_all(&self.pool)
        .await;

        if let Ok(events) = rows {
            for row in events {
                let id: Uuid = row.get("id");
                let event_type: String = row.get("event_type");
                let payload: serde_json::Value = row.get("payload");

                // Simulated HTTP Webhook Dispatch
                info!("Outbox Dispatching: {} -> {}", event_type, payload);
                
                // If it succeeds, mark as completed
                sqlx::query("UPDATE outbox_events SET status = 'COMPLETED', processed_at = NOW() WHERE id = $1")
                    .bind(id)
                    .execute(&self.pool)
                    .await
                    .ok();
            }
        } else if let Err(e) = rows {
            error!("Outbox worker failed to fetch events: {}", e);
        }
    }
}
