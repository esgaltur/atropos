use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use crate::domain::{PoolId, AllocationPolicy};

/// Represents a collection of similar resources managed under a specific allocation policy.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pool {
    /// Unique identifier for the pool.
    pub id: PoolId,
    /// Human-readable name of the pool.
    pub name: String,
    /// The type of resources contained in this pool (e.g., "A100-GPU").
    pub resource_type: String,
    /// The policy governing how resources are allocated from this pool.
    pub policy: AllocationPolicy,
    /// Timestamp when the pool was created.
    pub created_at: DateTime<Utc>,
}

impl Pool {
    // TODO(esgaltur): I'm thinking about adding a `max_capacity` field here 
    // to limit how many resources can be added to a pool.
    /// Creates a new resource pool with a unique ID and current timestamp.
    pub fn new(name: String, resource_type: String, policy: AllocationPolicy) -> Self {
        Self {
            id: PoolId::new(),
            name,
            resource_type,
            policy,
            created_at: Utc::now(),
        }
    }
}
