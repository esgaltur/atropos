use crate::domain::{error::DomainError, pool::Pool, repository::PoolRepository, AllocationPolicy};
use std::sync::Arc;

#[derive(Clone)]
pub struct PoolService<R: PoolRepository> {
    repo: Arc<R>,
}

impl<R: PoolRepository> PoolService<R> {
    pub fn new(repo: Arc<R>) -> Self {
        Self { repo }
    }

    pub async fn create_pool(
        &self,
        name: String,
        resource_type: String,
        policy: AllocationPolicy,
        max_capacity: Option<i32>,
    ) -> Result<Pool, DomainError> {
        let pool = Pool::new(name, resource_type, policy).with_max_capacity(max_capacity);
        self.repo.create(pool.clone()).await?;
        Ok(pool)
    }

    pub async fn find_pool_by_name(&self, name: &str) -> Result<Option<Pool>, DomainError> {
        self.repo.find_by_name(name).await
    }
}
