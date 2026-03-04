use chrono::{DateTime, Utc, Duration};
use serde::{Deserialize, Serialize};
use crate::domain::{LeaseId, ResourceId, LeaseStatus};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Lease {
    pub id: LeaseId,
    pub resource_id: ResourceId,
    pub owner_id: String,
    pub tenant_id: String,
    pub status: LeaseStatus,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub idempotency_key: Option<String>,
    pub cost_center: Option<String>,
}

impl Lease {
    pub fn new(
        resource_id: ResourceId,
        owner_id: String,
        tenant_id: String,
        ttl_seconds: i64,
        idempotency_key: Option<String>,
        cost_center: Option<String>,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: LeaseId::new(),
            resource_id,
            owner_id,
            tenant_id,
            status: LeaseStatus::Active,
            created_at: now,
            expires_at: now + Duration::seconds(ttl_seconds),
            idempotency_key,
            cost_center,
        }
    }

    /// Pure function to check if the lease is expired based on a provided timestamp.
    /// This supports the DIP by not relying on the system clock inside the logic.
    ///
    /// # Examples
    ///
    /// ```
    /// use atropos::domain::lease::Lease;
    /// use atropos::domain::{ResourceId, LeaseStatus};
    /// use chrono::{Utc, Duration};
    ///
    /// let lease = Lease::new(ResourceId::new(), "o".into(), "t".into(), 60, None, None);
    /// assert_eq!(lease.is_expired(Utc::now() + Duration::seconds(70)), true);
    /// ```
    pub fn is_expired(&self, current_time: DateTime<Utc>) -> bool {
        current_time >= self.expires_at && self.status == LeaseStatus::Active
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lease_is_not_expired_before_time() {
        let resource_id = ResourceId::new();
        let ttl = 60;
        let lease = Lease::new(
            resource_id,
            "user1".to_string(),
            "tenant1".to_string(),
            ttl,
            None,
            None,
        );

        let check_time = Utc::now() + Duration::seconds(30);
        assert!(!lease.is_expired(check_time));
    }

    #[test]
    fn test_lease_is_expired_after_time() {
        let resource_id = ResourceId::new();
        let ttl = 60;
        let lease = Lease::new(
            resource_id,
            "user1".to_string(),
            "tenant1".to_string(),
            ttl,
            None,
            None,
        );

        let check_time = Utc::now() + Duration::seconds(61);
        assert!(lease.is_expired(check_time));
    }

    #[test]
    fn test_released_lease_is_never_expired() {
        let mut lease = Lease::new(
            ResourceId::new(),
            "user1".to_string(),
            "tenant1".to_string(),
            60,
            None,
            None,
        );
        lease.status = LeaseStatus::Released;
        
        let check_time = Utc::now() + Duration::seconds(100);
        // Even if the clock is past expires_at, if it's already RELEASED, it shouldn't be "Expired"
        assert!(!lease.is_expired(check_time));
    }
}
