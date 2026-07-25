use hmac::{Hmac, Mac};
use sha2::Sha256;
use sqlx::{PgPool, Row};
use std::time::Duration;
use tokio::time;
use tracing::{error, info};
use uuid::Uuid;

type HmacSha256 = Hmac<Sha256>;

/// Maximum delivery attempts before an event is dead-lettered (status `FAILED`).
const MAX_ATTEMPTS: i32 = 5;

/// Environment variable holding the shared secret used to HMAC-sign webhook bodies.
const WEBHOOK_SECRET_ENV: &str = "ATROPOS_WEBHOOK_SECRET";

pub struct OutboxService {
    pool: PgPool,
    client: reqwest::Client,
}

impl OutboxService {
    pub fn new(pool: PgPool) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .unwrap_or_default();
        Self { pool, client }
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
            RETURNING outbox_events.id, outbox_events.event_type, outbox_events.payload, outbox_events.attempts
            "#,
        )
        .fetch_all(&self.pool)
        .await;

        match rows {
            Ok(events) => {
                for row in events {
                    let id: Uuid = row.get("id");
                    let event_type: String = row.get("event_type");
                    let payload: serde_json::Value = row.get("payload");
                    let attempts: i32 = row.get("attempts");

                    match self.dispatch_event(&event_type, &payload).await {
                        Ok(()) => {
                            info!("Outbox delivered: {} -> {}", event_type, payload);
                            sqlx::query(
                                "UPDATE outbox_events SET status = 'COMPLETED', processed_at = NOW() WHERE id = $1",
                            )
                            .bind(id)
                            .execute(&self.pool)
                            .await
                            .ok();
                        }
                        Err(err) => {
                            let new_attempts = attempts + 1;
                            if new_attempts >= MAX_ATTEMPTS {
                                error!(
                                    "Outbox event {} dead-lettered (FAILED) after {} attempts: {}",
                                    id, new_attempts, err
                                );
                                sqlx::query(
                                    "UPDATE outbox_events SET status = 'FAILED', attempts = $2, last_error = $3, processed_at = NOW() WHERE id = $1",
                                )
                                .bind(id)
                                .bind(new_attempts)
                                .bind(&err)
                                .execute(&self.pool)
                                .await
                                .ok();
                            } else {
                                error!(
                                    "Outbox event {} delivery failed (attempt {}): {}. Requeuing.",
                                    id, new_attempts, err
                                );
                                sqlx::query(
                                    "UPDATE outbox_events SET status = 'PENDING', attempts = $2, last_error = $3 WHERE id = $1",
                                )
                                .bind(id)
                                .bind(new_attempts)
                                .bind(&err)
                                .execute(&self.pool)
                                .await
                                .ok();
                            }
                        }
                    }
                }
            }
            Err(e) => error!("Outbox worker failed to fetch events: {}", e),
        }
    }

    /// Delivers an event to every webhook subscribed to its type.
    ///
    /// Subscriptions are matched on `event_type` (an exact match or the wildcard
    /// `*`). When the event payload carries a `tenant_id`, delivery is further
    /// scoped to that tenant's webhooks (plus any wildcard-tenant subscriptions).
    /// If no webhooks are configured the event is treated as delivered so it does
    /// not clog the queue. Any HTTP failure returns `Err` so the caller applies
    /// the retry / dead-letter policy.
    async fn dispatch_event(
        &self,
        event_type: &str,
        payload: &serde_json::Value,
    ) -> Result<(), String> {
        let tenant_id = payload.get("tenant_id").and_then(|v| v.as_str());

        let webhooks = sqlx::query(
            r#"
            SELECT url, tenant_id
            FROM webhooks
            WHERE (event_type = $1 OR event_type = '*')
              AND ($2::text IS NULL OR tenant_id = $2 OR tenant_id = '*')
            "#,
        )
        .bind(event_type)
        .bind(tenant_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| format!("failed to load webhooks: {e}"))?;

        if webhooks.is_empty() {
            return Ok(());
        }

        let body = serde_json::to_vec(payload).map_err(|e| e.to_string())?;
        let signature = sign_payload(&body);

        for row in webhooks {
            let url: String = row.get("url");
            let mut request = self
                .client
                .post(&url)
                .header("Content-Type", "application/json")
                .header("X-Atropos-Event", event_type);
            if let Some(sig) = &signature {
                request = request.header("X-Atropos-Signature", sig);
            }

            let response = request
                .body(body.clone())
                .send()
                .await
                .map_err(|e| format!("webhook POST to {url} failed: {e}"))?;

            if !response.status().is_success() {
                return Err(format!(
                    "webhook {url} responded with status {}",
                    response.status()
                ));
            }
        }

        Ok(())
    }
}

/// Produces a lowercase hex HMAC-SHA256 signature of `body` using the configured
/// secret, or `None` when no secret is set (unsigned delivery in dev).
fn sign_payload(body: &[u8]) -> Option<String> {
    let secret = std::env::var(WEBHOOK_SECRET_ENV).ok()?;
    if secret.is_empty() {
        return None;
    }
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).ok()?;
    mac.update(body);
    Some(hex::encode(mac.finalize().into_bytes()))
}
