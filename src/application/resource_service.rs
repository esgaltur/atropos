use std::sync::Arc;
use serde_json::Value;
use crate::domain::{
    resource::Resource,
    error::DomainError,
    repository::ResourceRepository,
    PoolId,
};

#[derive(Clone)]
pub struct ResourceService<R: ResourceRepository> {
    repo: Arc<R>,
}

impl<R: ResourceRepository> ResourceService<R> {
    pub fn new(repo: Arc<R>) -> Self {
        Self { repo }
    }

    pub async fn register_resource(&self, pool_id: PoolId, external_id: String, attributes: Value) -> Result<Resource, DomainError> {
        let resource = Resource::new(pool_id, external_id, attributes);
        self.repo.create(resource.clone()).await?;
        Ok(resource)
    }
}
