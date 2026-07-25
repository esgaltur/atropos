use thiserror::Error;

#[derive(Error, Debug)]
pub enum DomainError {
    #[error("Resource not found")]
    ResourceNotFound,
    #[error("Pool not found")]
    PoolNotFound,
    #[error("Lease not found")]
    LeaseNotFound,
    #[error("No resources available for allocation")]
    NoResourcesAvailable,
    #[error("Tenant quota exceeded: {0}")]
    QuotaExceeded(String),
    #[error("Infrastructure error: {0}")]
    InfrastructureError(String),
}
