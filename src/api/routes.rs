use crate::application::{
    allocation_service::AllocationService, platform_service::PlatformService,
    pool_service::PoolService, resource_service::ResourceService,
};
use crate::domain::repository::{
    AllocationRepository, PlatformRepository, PoolRepository, ResourceRepository,
};
use axum::{
    middleware,
    routing::{delete, get, patch, post, put},
    Router,
};
use std::sync::Arc;

use super::{auth, handlers, ui};

pub trait AppRepository:
    AllocationRepository
    + PoolRepository
    + ResourceRepository
    + PlatformRepository
    + Send
    + Sync
    + 'static
{
}

impl<T> AppRepository for T where
    T: AllocationRepository
        + PoolRepository
        + ResourceRepository
        + PlatformRepository
        + Send
        + Sync
        + 'static
{
}

pub struct AppState<R: AppRepository> {
    pub pool_service: Arc<PoolService<R>>,
    pub resource_service: Arc<ResourceService<R>>,
    pub allocation_service: Arc<AllocationService<R>>,
    pub platform_service: Arc<PlatformService<R>>,
}

impl<R: AppRepository> Clone for AppState<R> {
    fn clone(&self) -> Self {
        Self {
            pool_service: self.pool_service.clone(),
            resource_service: self.resource_service.clone(),
            allocation_service: self.allocation_service.clone(),
            platform_service: self.platform_service.clone(),
        }
    }
}

pub fn create_router<R: AppRepository>(state: AppState<R>) -> Router {
    // Mutating endpoints require a bearer token when ATROPOS_API_TOKEN is configured.
    let protected = Router::new()
        .route("/pools", post(handlers::create_pool))
        .route("/resources", post(handlers::register_resource))
        .route(
            "/resources/:id/status",
            patch(handlers::update_resource_status),
        )
        .route("/leases", post(handlers::allocate_lease))
        .route("/leases/:id", delete(handlers::release_lease))
        .route("/leases/:id/renew", post(handlers::renew_lease))
        .route("/leases/:id/heartbeat", post(handlers::heartbeat_lease))
        .route("/leases/:id/labels", patch(handlers::set_lease_labels))
        .route("/quotas", put(handlers::set_quota))
        .route("/reservations", post(handlers::create_reservation))
        .route_layer(middleware::from_fn(auth::require_bearer_token));

    // Read-only dashboard, health, and lookup endpoints remain public.
    let public = Router::new()
        .route("/", get(ui::admin_dashboard))
        .route("/admin", get(ui::admin_dashboard))
        .route("/admin/audit-log", get(ui::audit_log_stream))
        .route("/health", get(handlers::health_check))
        .route("/pools/:name", get(handlers::get_pool_by_name))
        .route(
            "/pools/:name/utilization",
            get(handlers::get_pool_utilization),
        )
        .route("/leases", get(handlers::list_leases))
        .route("/leases/:id", get(handlers::get_lease))
        .route("/quotas/:tenant_id/:pool_type", get(handlers::get_quota))
        .route("/waitlist/:id", get(handlers::get_waitlist_position))
        .route("/reservations/:id", get(handlers::get_reservation))
        .route("/reports/cost", get(handlers::cost_report));

    public.merge(protected).with_state(state)
}
