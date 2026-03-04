use tonic::{Request, Response, Status};
use std::sync::Arc;
use crate::api::grpc::atropos_v1::allocation_service_server::AllocationService;
use crate::api::grpc::atropos_v1::{AllocateRequest, AllocateResponse};
use crate::application::allocation_service::AllocationService as AppService;
use crate::domain::repository::{AllocationRepository, PoolRepository};

// Import the generated code
pub mod atropos_v1 {
    tonic::include_proto!("atropos.v1");
}

pub struct GrpcAllocationService<R: AllocationRepository + PoolRepository> {
    app_service: Arc<AppService<R>>,
}

impl<R: AllocationRepository + PoolRepository> GrpcAllocationService<R> {
    pub fn new(app_service: Arc<AppService<R>>) -> Self {
        Self { app_service }
    }
}

#[tonic::async_trait]
impl<R: AllocationRepository + PoolRepository + 'static> AllocationService for GrpcAllocationService<R> {
    async fn allocate(
        &self,
        request: Request<AllocateRequest>,
    ) -> Result<Response<AllocateResponse>, Status> {
        let req = request.into_inner();
        
        let result = self.app_service.allocate(
            req.pool_type,
            req.owner_id,
            req.tenant_id,
            req.ttl_seconds,
            req.idempotency_key,
            Some(req.waitlist),
            Some(req.preempt),
        ).await;

        match result {
            Ok(lease) => Ok(Response::new(AllocateResponse {
                lease_id: lease.id.to_string(),
                resource_id: lease.resource_id.to_string(),
                status: "ACTIVE".into(),
            })),
            Err(e) => Err(Status::internal(e.to_string())),
        }
    }
}
