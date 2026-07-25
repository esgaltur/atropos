use crate::domain::LeaseId;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

/// A request for capacity to be granted at a future point in time.
///
/// A background promoter attempts to allocate a real [`crate::domain::lease::Lease`]
/// once `start_at` is reached, transitioning the reservation from `PENDING` to
/// `FULFILLED` (or `FAILED` if capacity could not be obtained).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Reservation {
    pub id: Uuid,
    pub pool_type: String,
    pub owner_id: String,
    pub tenant_id: String,
    pub priority: i32,
    pub ttl_seconds: i64,
    pub constraints: Option<Value>,
    pub start_at: DateTime<Utc>,
    pub status: String,
    pub lease_id: Option<LeaseId>,
    pub created_at: DateTime<Utc>,
}
