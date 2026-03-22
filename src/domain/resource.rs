use crate::domain::{PoolId, ResourceId, ResourceStatus};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// An individual resource instance that can be leased by users or automated systems.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Resource {
    /// Unique identifier for the resource.
    pub id: ResourceId,
    /// Identifier of the pool this resource belongs to.
    pub pool_id: PoolId,
    /// An external reference ID (e.g., serial number, cloud instance ID).
    pub external_id: String,
    /// Current health or availability status of the resource.
    pub status: ResourceStatus,
    /// Arbitrary JSON metadata describing the resource's specific properties.
    pub attributes: Value,
    /// Optimistic concurrency version for database updates.
    pub version: i64,
    /// Timestamp of the last status or attribute update.
    pub updated_at: DateTime<Utc>,
}

impl Resource {
    /// Creates a new Resource instance with a default status of `Healthy`.
    pub fn new(pool_id: PoolId, external_id: String, attributes: Value) -> Self {
        Self {
            id: ResourceId::new(),
            pool_id,
            external_id,
            status: ResourceStatus::Healthy,
            attributes,
            version: 0,
            updated_at: Utc::now(),
        }
    }
}
