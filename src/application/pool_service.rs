use std::sync::Arc;
use crate::domain::{
    pool::Pool,
    error::DomainError,
    repository::PoolRepository,
    AllocationPolicy,
};

#[derive(Clone)]
pub struct PoolService<R: PoolRepository> {
    repo: Arc<R>,
}

impl<R: PoolRepository> PoolService<R> {
    pub fn new(repo: Arc<R>) -> Self {
        Self { repo }
    }

    pub async fn create_pool(&self, name: String, resource_type: String, policy: AllocationPolicy) -> Result<Pool, DomainError> {
        let pool = Pool::new(name, resource_type, policy);
        self.repo.create(pool.clone()).await?;
        Ok(pool)
    }
}
