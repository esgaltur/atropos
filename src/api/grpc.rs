use crate::api::grpc::atropos_v1::allocation_service_server::AllocationService;
use crate::api::grpc::atropos_v1::{AllocateRequest, AllocateResponse};
use crate::application::allocation_service::{AllocationOutcome, AllocationService as AppService};
use crate::domain::error::DomainError;
use crate::domain::repository::{AllocationRepository, PoolRepository};
use std::sync::Arc;
use tonic::{Request, Response, Status};

// Import the generated code
pub mod atropos_v1 {
    tonic::include_proto!("atropos.v1");
}

/// gRPC interceptor mirroring the REST bearer-token auth.
///
/// When `ATROPOS_API_TOKEN` is set, requests must carry an `authorization:
/// Bearer <token>` metadata entry; otherwise all requests are allowed (dev mode).
// The `Result<_, Status>` signature is mandated by tonic's interceptor contract.
#[allow(clippy::result_large_err)]
pub fn check_auth(req: Request<()>) -> Result<Request<()>, Status> {
    let expected = match crate::api::auth::configured_token() {
        Some(t) => t,
        None => return Ok(req),
    };

    let provided = req
        .metadata()
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "));

    match provided {
        Some(token)
            if crate::api::auth::constant_time_eq(token.as_bytes(), expected.as_bytes()) =>
        {
            Ok(req)
        }
        _ => Err(Status::unauthenticated("missing or invalid bearer token")),
    }
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
impl<R: AllocationRepository + PoolRepository + 'static> AllocationService
    for GrpcAllocationService<R>
{
    async fn allocate(
        &self,
        request: Request<AllocateRequest>,
    ) -> Result<Response<AllocateResponse>, Status> {
        let req = request.into_inner();

        // Parse optional JSON constraints, rejecting malformed input up-front.
        let constraints = match req.constraints.as_deref() {
            Some(raw) => Some(
                serde_json::from_str::<serde_json::Value>(raw)
                    .map_err(|e| Status::invalid_argument(format!("invalid constraints: {e}")))?,
            ),
            None => None,
        };

        let result = self
            .app_service
            .allocate(
                req.pool_type,
                req.owner_id,
                req.tenant_id,
                req.priority.unwrap_or(0),
                req.ttl_seconds,
                constraints,
                None, // spread_by
                req.idempotency_key,
                Some(req.waitlist),
                Some(req.preempt),
            )
            .await;

        match result {
            Ok(AllocationOutcome::Leased(lease)) => Ok(Response::new(AllocateResponse {
                lease_id: lease.id.to_string(),
                resource_id: lease.resource_id.to_string(),
                status: "ACTIVE".into(),
            })),
            Ok(AllocationOutcome::Waitlisted) => Ok(Response::new(AllocateResponse {
                lease_id: String::new(),
                resource_id: String::new(),
                status: "WAITLISTED".into(),
            })),
            Err(DomainError::NoResourcesAvailable) => {
                Err(Status::failed_precondition("no resources available"))
            }
            Err(DomainError::QuotaExceeded(msg)) => Err(Status::resource_exhausted(msg)),
            Err(e) => Err(Status::internal(e.to_string())),
        }
    }
}
