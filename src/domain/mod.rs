use serde::{Deserialize, Serialize};
use uuid::Uuid;
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PoolId(pub Uuid);

impl PoolId {
    pub fn new() -> Self { Self(Uuid::new_v4()) }
}

impl Default for PoolId {
    fn default() -> Self { Self::new() }
}

impl fmt::Display for PoolId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ResourceId(pub Uuid);

impl ResourceId {
    pub fn new() -> Self { Self(Uuid::new_v4()) }
}

impl Default for ResourceId {
    fn default() -> Self { Self::new() }
}

impl fmt::Display for ResourceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct LeaseId(pub Uuid);

impl LeaseId {
    pub fn new() -> Self { Self(Uuid::new_v4()) }
}

impl Default for LeaseId {
    fn default() -> Self { Self::new() }
}

impl fmt::Display for LeaseId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, strum::Display, strum::EnumString)]
pub enum AllocationPolicy {
    FIFO,
    LRU,
    Random,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, strum::Display, strum::EnumString)]
pub enum ResourceStatus {
    Healthy,
    Unhealthy,
    Draining,
    Disabled,
    Cooldown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, strum::Display, strum::EnumString)]
pub enum LeaseStatus {
    Pending,
    Active,
    Expiring,
    Released,
    Expired,
    Revoked,
    Waiting,
}

pub mod pool;
pub mod resource;
pub mod lease;
pub mod error;
pub mod repository;

