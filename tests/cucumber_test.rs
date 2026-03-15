use cucumber::{given, when, then, World};
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use std::future::Future;
use chrono::Utc;
use uuid::Uuid;
use atropos::domain::{
    pool::Pool, resource::Resource, lease::Lease,
    PoolId, ResourceId, LeaseId,
    LeaseStatus,
    error::DomainError, repository::{PoolRepository, ResourceRepository, AllocationRepository}
};
use atropos::application::allocation_service::AllocationService;
use atropos::domain::repository::{AuditLogEntry, SummaryStats};

// --- A Robust Mock Repository for Testing ---
#[derive(Default, Clone, Debug)]
struct MockRepository {
    available_resources: Arc<Mutex<i32>>,
    total_resources: Arc<Mutex<i32>>,
    leases: Arc<Mutex<HashMap<String, Lease>>>, // By ID string
    idempotency_map: Arc<Mutex<HashMap<String, Lease>>>, // By Key string
    audit_logs: Arc<Mutex<Vec<AuditLogEntry>>>,
    waitlist_count: Arc<Mutex<i32>>,
}

impl PoolRepository for MockRepository {
    fn create(&self, _pool: Pool) -> impl Future<Output = Result<(), DomainError>> + Send { async { Ok(()) } }
    fn find_by_id(&self, _id: &PoolId) -> impl Future<Output = Result<Option<Pool>, DomainError>> + Send { async { Ok(None) } }
}
impl ResourceRepository for MockRepository {
    fn create(&self, _res: Resource) -> impl Future<Output = Result<(), DomainError>> + Send { async { Ok(()) } }
    fn find_by_id(&self, _id: &ResourceId) -> impl Future<Output = Result<Option<Resource>, DomainError>> + Send { async { Ok(None) } }
}
impl AllocationRepository for MockRepository {
    fn allocate_resource(&self, _pool: String, owner: String, tenant: String, ttl: i64, key: Option<String>, _cost: Option<String>) -> impl Future<Output = Result<Lease, DomainError>> + Send {
        let available_ref = self.available_resources.clone();
        let leases_ref = self.leases.clone();
        let idem_ref = self.idempotency_map.clone();
        let audit_ref = self.audit_logs.clone();
        
        async move {
            // Check Idempotency
            if let Some(ref k) = key {
                let map = idem_ref.lock().unwrap();
                if let Some(existing) = map.get(k) {
                    return Ok(existing.clone());
                }
            }

            let mut available = available_ref.lock().unwrap();
            if *available > 0 {
                *available -= 1;
                let lease = Lease {
                    id: LeaseId::new(),
                    resource_id: ResourceId::new(),
                    owner_id: owner.clone(),
                    tenant_id: tenant,
                    status: LeaseStatus::Active,
                    created_at: Utc::now(),
                    expires_at: Utc::now() + chrono::Duration::seconds(ttl),
                    idempotency_key: key.clone(),
                    cost_center: None,
                };
                
                leases_ref.lock().unwrap().insert(lease.id.to_string(), lease.clone());
                if let Some(k) = key {
                    idem_ref.lock().unwrap().insert(k, lease.clone());
                }

                // Add audit log
                audit_ref.lock().unwrap().push(AuditLogEntry {
                    id: 1_i64,
                    actor_id: Some(owner),
                    action: Some("ALLOCATE".to_string()),
                    resource_id: Some(lease.resource_id.0),
                    created_at: Utc::now(),
                });

                Ok(lease)
            } else {
                Err(DomainError::NoResourcesAvailable)
            }
        }
    }
    fn release_lease(&self, id: &LeaseId) -> impl Future<Output = Result<(), DomainError>> + Send {
        let leases_ref = self.leases.clone();
        let id_str = id.to_string();
        async move {
            let mut map = leases_ref.lock().unwrap();
            if map.contains_key(&id_str) {
                map.remove(&id_str);
                Ok(())
            } else {
                Err(DomainError::LeaseNotFound)
            }
        }
    }
    fn renew_lease(&self, id: &LeaseId, _sec: i64) -> impl Future<Output = Result<(), DomainError>> + Send {
        let leases_ref = self.leases.clone();
        let id_str = id.to_string();
        async move {
            let map = leases_ref.lock().unwrap();
            if map.contains_key(&id_str) {
                Ok(())
            } else {
                Err(DomainError::LeaseNotFound)
            }
        }
    }

    fn waitlist_resource(
        &self,
        _pool_type: String,
        _owner_id: String,
        _tenant_id: String,
        _priority: i32
    ) -> impl Future<Output = Result<(), DomainError>> + Send {
        let wait_ref = self.waitlist_count.clone();
        async move {
            let mut count = wait_ref.lock().unwrap();
            *count += 1;
            Ok(())
        }
    }

    fn get_summary_stats(&self) -> impl Future<Output=Result<SummaryStats, DomainError>> + Send {
        let leases = self.leases.clone();
        let total = self.total_resources.clone();
        let wait = self.waitlist_count.clone();
        async move {
            let total_val = *total.lock().unwrap() as i64;
            Ok(SummaryStats {
                active_leases: leases.lock().unwrap().len() as i64,
                total_resources: total_val,
                healthy_resources: total_val, // Mock assumes all resources are healthy
                waitlist_count: *wait.lock().unwrap() as i64,
            })
        }
    }

    fn get_recent_audit_logs(&self, limit: i64) -> impl Future<Output=Result<Vec<AuditLogEntry>, DomainError>> + Send {
        let logs = self.audit_logs.clone();
        async move {
            let logs = logs.lock().unwrap();
            let count = (limit as usize).min(logs.len());
            Ok(logs[logs.len() - count..].to_vec())
        }
    }
}

// --- Cucumber World ---
#[derive(Debug, World, Default)]
struct AtroposWorld {
    repo: MockRepository,
    last_results: Vec<Result<Lease, DomainError>>,
    last_op_result: Option<Result<(), DomainError>>,
    last_stats: Option<SummaryStats>,
    last_logs: Vec<AuditLogEntry>,
}

#[given(expr = "a resource pool {string} exists")]
fn pool_exists(_world: &mut AtroposWorld, _name: String) {}

#[given(expr = "the pool has {int} {string} GPU resource")]
#[given(expr = "the pool has {int} {string} GPU resources")]
fn set_available(world: &mut AtroposWorld, count: i32, _status: String) {
    *world.repo.available_resources.lock().unwrap() = count;
    *world.repo.total_resources.lock().unwrap() = count;
}

#[when(expr = "a research team {string} requests 1 GPU")]
#[when(expr = "a team {string} requests 1 GPU")]
async fn request_gpu(world: &mut AtroposWorld, team: String) {
    let service = AllocationService::new(Arc::new(world.repo.clone()));
    let res = service.allocate("GPU".into(), "user-01".into(), team, 60, None, None, None).await;
    world.last_results.push(res);
}

#[when(expr = "a team {string} allocates a GPU")]
async fn team_allocates_gpu(world: &mut AtroposWorld, team: String) {
    request_gpu(world, team).await;
}

#[when(expr = "a team {string} requests a GPU with idempotency key {string}")]
async fn request_gpu_idem(world: &mut AtroposWorld, team: String, key: String) {
    let service = AllocationService::new(Arc::new(world.repo.clone()));
    let res = service.allocate("GPU".into(), "user-01".into(), team, 60, Some(key), None, None).await;
    world.last_results.push(res);
}

#[when(expr = "the same team {string} requests a GPU with the same key {string}")]
async fn request_gpu_idem_same(world: &mut AtroposWorld, team: String, key: String) {
    request_gpu_idem(world, team, key).await;
}

#[when(expr = "a team {string} requests a GPU with waitlist enabled")]
async fn request_gpu_waitlist(world: &mut AtroposWorld, team: String) {
    let service = AllocationService::new(Arc::new(world.repo.clone()));
    let res = service.allocate("GPU".into(), "user-01".into(), team, 60, None, Some(true), None).await;
    world.last_results.push(res);
}

#[then(expr = "the allocation should be {string}")]
fn check_result(world: &mut AtroposWorld, expected: String) {
    let actual = world.last_results.last().unwrap();
    match expected.as_str() {
        "Successful" => assert!(actual.is_ok()),
        "Denied" => assert!(actual.is_err()),
        _ => panic!("Unknown expectation"),
    }
}

#[then(expr = "a unique Lease should be issued")]
fn check_lease(world: &mut AtroposWorld) {
    assert!(world.last_results.last().unwrap().is_ok());
}

#[then(expr = "the reason should be {string}")]
fn check_reason(world: &mut AtroposWorld, reason: String) {
    if let Some(res) = world.last_results.last() {
        if let Err(err) = res {
             if err.to_string() == reason { return; }
        }
    }
    
    if let Some(res) = &world.last_op_result {
        if let Err(err) = res {
             assert_eq!(err.to_string(), reason);
             return;
        }
    }
    panic!("No error found matching reason: {}", reason);
}

#[then(expr = "both responses should contain the {string} Lease ID")]
fn check_idem(world: &mut AtroposWorld, match_type: String) {
    let l1 = world.last_results.get(world.last_results.len() - 2).unwrap().as_ref().unwrap();
    let l2 = world.last_results.last().unwrap().as_ref().unwrap();
    if match_type == "Same" {
        assert_eq!(l1.id, l2.id);
    }
}

// --- Lifecycle Steps ---
#[given(expr = "a research team has an {string} lease for a GPU")]
async fn given_active_lease(world: &mut AtroposWorld, _status: String) {
    *world.repo.available_resources.lock().unwrap() = 1;
    *world.repo.total_resources.lock().unwrap() = 1;
    request_gpu(world, "team-01".into()).await;
}

#[when(expr = "they request a renewal for {int} seconds")]
async fn renew_lease(world: &mut AtroposWorld, sec: i32) {
    let lease = world.last_results.last().unwrap().as_ref().unwrap();
    let service = AllocationService::new(Arc::new(world.repo.clone()));
    let res = service.renew(lease.id, sec as i64).await;
    world.last_op_result = Some(res);
}

#[then(expr = "the renewal should be {string}")]
fn check_renew_result(world: &mut AtroposWorld, expected: String) {
    let actual = world.last_op_result.as_ref().unwrap();
    if expected == "Successful" {
        assert!(actual.is_ok());
    }
}

#[given(expr = "no active lease exists for ID {string}")]
fn no_lease(_world: &mut AtroposWorld, _id: String) {}

#[when(expr = "they attempt to release a non-existent lease")]
async fn release_non_existent(world: &mut AtroposWorld) {
    let service = AllocationService::new(Arc::new(world.repo.clone()));
    let res = service.release(LeaseId::new()).await;
    world.last_op_result = Some(res);
}

#[when(expr = "they release the active lease")]
async fn release_active_lease(world: &mut AtroposWorld) {
    let lease = world.last_results.last().unwrap().as_ref().unwrap();
    let service = AllocationService::new(Arc::new(world.repo.clone()));
    let res = service.release(lease.id).await;
    world.last_op_result = Some(res);
}

#[then(expr = "the release should be {string}")]
fn check_release_result(world: &mut AtroposWorld, expected: String) {
    let actual = world.last_op_result.as_ref().unwrap();
    if expected == "Successful" {
        assert!(actual.is_ok());
    }
}

#[then(expr = "the result should be {string}")]
fn check_op_result(world: &mut AtroposWorld, expected: String) {
    let actual = world.last_op_result.as_ref().unwrap();
    if expected == "Not Found" {
        assert!(matches!(actual, Err(DomainError::LeaseNotFound)));
    }
}

// --- Observability Steps ---
#[given(expr = "the resource pool {string} exists with {int} healthy resources")]
fn pool_exists_with_resources(world: &mut AtroposWorld, _name: String, count: i32) {
    *world.repo.available_resources.lock().unwrap() = count;
    *world.repo.total_resources.lock().unwrap() = count;
}

#[given(expr = "there is {int} active lease for {string}")]
async fn given_n_active_leases(world: &mut AtroposWorld, count: i32, team: String) {
    for _ in 0..count {
        request_gpu(world, team.clone()).await;
    }
}

#[when(expr = "the administrator requests summary statistics")]
async fn admin_requests_stats(world: &mut AtroposWorld) {
    let service = AllocationService::new(Arc::new(world.repo.clone()));
    world.last_stats = Some(service.get_stats().await.unwrap());
}

#[then(expr = "the active lease count should be {int}")]
fn check_active_leases(world: &mut AtroposWorld, expected: i32) {
    assert_eq!(world.last_stats.as_ref().unwrap().active_leases, expected as i64);
}

#[then(expr = "the total healthy resource count should be {int}")]
fn check_healthy_resources(world: &mut AtroposWorld, expected: i32) {
    assert_eq!(world.last_stats.as_ref().unwrap().healthy_resources, expected as i64);
}

#[then(expr = "the waitlist count should be {int}")]
fn check_waitlist_count(world: &mut AtroposWorld, expected: i32) {
    assert_eq!(world.last_stats.as_ref().unwrap().waitlist_count, expected as i64);
}

#[when(expr = "the administrator requests recent audit logs")]
async fn admin_requests_logs(world: &mut AtroposWorld) {
    let service = AllocationService::new(Arc::new(world.repo.clone()));
    world.last_logs = service.get_recent_logs(10).await.unwrap();
}

#[then(expr = "the latest audit log should show {string}")]
fn check_latest_log_action(world: &mut AtroposWorld, action: String) {
    assert_eq!(world.last_logs.last().unwrap().action.as_deref(), Some(action.as_str()));
}

#[then(expr = "the logs should contain at least {int} entry")]
#[then(expr = "the logs should contain at least {int} entries")]
fn check_log_count(world: &mut AtroposWorld, count: i32) {
    assert!(world.last_logs.len() >= count as usize);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_cucumber_features() {
        // Run both feature files
        AtroposWorld::run("tests/features/allocation.feature").await;
        AtroposWorld::run("tests/features/lifecycle.feature").await;
        AtroposWorld::run("tests/features/observability.feature").await;
    }
}
