use cucumber::{given, when, then, World};
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use std::future::Future;
use chrono::Utc;
use atropos::domain::{
    pool::Pool, resource::Resource, lease::Lease,
    PoolId, ResourceId, LeaseId,
    LeaseStatus,
    error::DomainError, repository::{PoolRepository, ResourceRepository, AllocationRepository}
};
use atropos::application::allocation_service::AllocationService;

// --- A Robust Mock Repository for Testing ---
#[derive(Default, Clone, Debug)]
struct MockRepository {
    available_resources: Arc<Mutex<i32>>,
    leases: Arc<Mutex<HashMap<String, Lease>>>, // By ID string
    idempotency_map: Arc<Mutex<HashMap<String, Lease>>>, // By Key string
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
                    owner_id: owner,
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
        async move {
            // Mock always succeeds in waitlisting for tests
            Ok(())
        }
    }
}

// --- Cucumber World ---
#[derive(Debug, World, Default)]
struct AtroposWorld {
    repo: MockRepository,
    last_results: Vec<Result<Lease, DomainError>>,
    last_op_result: Option<Result<(), DomainError>>,
}

#[given(expr = "a resource pool {string} exists")]
fn pool_exists(_world: &mut AtroposWorld, _name: String) {}

#[given(expr = "the pool has {int} {string} GPU resource")]
#[given(expr = "the pool has {int} {string} GPU resources")]
fn set_available(world: &mut AtroposWorld, count: i32, _status: String) {
    *world.repo.available_resources.lock().unwrap() = count;
}

#[when(expr = "a research team {string} requests 1 GPU")]
#[when(expr = "a team {string} requests 1 GPU")]
async fn request_gpu(world: &mut AtroposWorld, team: String) {
    let service = AllocationService::new(Arc::new(world.repo.clone()));
    let res = service.allocate("GPU".into(), "user-01".into(), team, 60, None, None, None).await;
    world.last_results.push(res);
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

#[then(expr = "the result should be {string}")]
fn check_op_result(world: &mut AtroposWorld, expected: String) {
    let actual = world.last_op_result.as_ref().unwrap();
    if expected == "Not Found" {
        assert!(matches!(actual, Err(DomainError::LeaseNotFound)));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_cucumber_features() {
        // Run both feature files
        AtroposWorld::run("tests/features/allocation.feature").await;
        AtroposWorld::run("tests/features/lifecycle.feature").await;
    }
}
