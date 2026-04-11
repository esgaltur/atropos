use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AllocationPolicy {
    FIFO,
    LRU,
    Random,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResourceStatus {
    Healthy,
    Unhealthy,
    Draining,
    Disabled,
    Cooldown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum LeaseStatus {
    Pending,
    Active,
    Expiring,
    Released,
    Expired,
    Revoked,
    Waiting,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatePoolRequest {
    pub name: String,
    pub resource_type: String,
    pub policy: AllocationPolicy,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterResourceRequest {
    pub pool_id: Uuid,
    pub external_id: String,
    pub attributes: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AllocateRequest {
    pub pool_type: String,
    pub constraints: Option<serde_json::Value>,
    pub spread_by: Option<String>, // e.g., "rack_id"
    pub priority: Option<i32>,
    pub ttl_seconds: i64,
    pub idempotency_key: Option<String>,
    pub waitlist: Option<bool>,
    pub preempt: Option<bool>,
    pub owner_id: String,
    pub tenant_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenewRequest {
    pub extension_seconds: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolResponse {
    pub id: Uuid,
    pub name: String,
    pub resource_type: String,
    pub policy: AllocationPolicy,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceResponse {
    pub id: Uuid,
    pub pool_id: Uuid,
    pub external_id: String,
    pub status: ResourceStatus,
    pub attributes: serde_json::Value,
    pub version: i64,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeaseResponse {
    pub id: Uuid,
    pub resource_id: Uuid,
    pub owner_id: String,
    pub tenant_id: String,
    pub status: LeaseStatus,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub idempotency_key: Option<String>,
    pub cost_center: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardStats {
    pub active_leases: i64,
    pub healthy_resources: i64,
    pub waitlist_count: i64,
    pub total_resources: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLogItem {
    pub created_at: String,
    pub action: String,
    pub actor_id: String,
    pub id: i64,
}
