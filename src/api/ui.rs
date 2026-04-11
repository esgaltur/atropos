use crate::api::routes::{AppRepository, AppState};
use atropos_contracts::{AuditLogItem, DashboardStats};
use atropos_frontend::{render_audit_log, render_dashboard};
use axum::{
    extract::State,
    http::StatusCode,
    response::{Html, IntoResponse},
};

const RECENT_LOG_LIMIT: i64 = 15;
const DEFAULT_ACTION: &str = "OP";
const DEFAULT_ACTOR: &str = "-";

fn map_stats(stats: crate::domain::repository::SummaryStats) -> DashboardStats {
    DashboardStats {
        active_leases: stats.active_leases,
        healthy_resources: stats.healthy_resources,
        waitlist_count: stats.waitlist_count,
        total_resources: stats.total_resources,
    }
}

fn map_log(log: crate::domain::repository::AuditLogEntry) -> AuditLogItem {
    AuditLogItem {
        created_at: log.created_at.format("%H:%M:%S").to_string(),
        action: log.action.unwrap_or_else(|| DEFAULT_ACTION.to_string()),
        actor_id: log.actor_id.unwrap_or_else(|| DEFAULT_ACTOR.to_string()),
        id: log.id,
    }
}

async fn fetch_stats_or_default<R: AppRepository>(
    state: &AppState<R>,
) -> crate::domain::repository::SummaryStats {
    state
        .allocation_service
        .get_stats()
        .await
        .unwrap_or_default()
}

async fn fetch_recent_logs<R: AppRepository>(state: &AppState<R>) -> Vec<AuditLogItem> {
    state
        .allocation_service
        .get_recent_logs(RECENT_LOG_LIMIT)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(map_log)
        .collect::<Vec<_>>()
}

pub async fn admin_dashboard(
    State(state): State<AppState<impl AppRepository>>,
) -> impl IntoResponse {
    let stats = fetch_stats_or_default(&state).await;
    let logs = fetch_recent_logs(&state).await;

    Html(render_dashboard(map_stats(stats), logs)).into_response()
}

pub async fn audit_log_stream(
    State(state): State<AppState<impl AppRepository>>,
) -> impl IntoResponse {
    let logs = fetch_recent_logs(&state).await;

    let html = render_audit_log(logs);
    if html.is_empty() {
        return (StatusCode::INTERNAL_SERVER_ERROR, "Log error").into_response();
    }
    Html(html).into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        application::{
            allocation_service::AllocationService, pool_service::PoolService,
            resource_service::ResourceService,
        },
        domain::{
            error::DomainError,
            lease::Lease,
            pool::Pool,
            repository::{
                AllocationRepository, AuditLogEntry, PoolRepository, ResourceRepository,
                SummaryStats,
            },
            resource::Resource,
            LeaseId, PoolId, ResourceId,
        },
    };
    use axum::body::to_bytes;
    use chrono::Utc;
    use std::{future::ready, sync::Arc};

    struct FakeRepository {
        stats: Option<SummaryStats>,
        logs: Option<Vec<AuditLogEntry>>,
    }

    impl PoolRepository for FakeRepository {
        fn create(
            &self,
            _pool: Pool,
        ) -> impl std::future::Future<Output = Result<(), DomainError>> + Send {
            ready(Ok(()))
        }

        fn find_by_id(
            &self,
            _id: &PoolId,
        ) -> impl std::future::Future<Output = Result<Option<Pool>, DomainError>> + Send {
            ready(Ok(None))
        }
    }

    impl ResourceRepository for FakeRepository {
        fn create(
            &self,
            _resource: Resource,
        ) -> impl std::future::Future<Output = Result<(), DomainError>> + Send {
            ready(Ok(()))
        }

        fn find_by_id(
            &self,
            _id: &ResourceId,
        ) -> impl std::future::Future<Output = Result<Option<Resource>, DomainError>> + Send
        {
            ready(Ok(None))
        }
    }

    impl AllocationRepository for FakeRepository {
        fn allocate_resource(
            &self,
            _pool_type: String,
            _owner_id: String,
            _tenant_id: String,
            _priority: i32,
            _ttl_seconds: i64,
            _constraints: Option<serde_json::Value>,
            _spread_by: Option<String>,
            _idempotency_key: Option<String>,
            _cost_center: Option<String>,
        ) -> impl std::future::Future<Output = Result<Lease, DomainError>> + Send {
            ready(Err(DomainError::InfrastructureError(
                "not used in ui tests".to_string(),
            )))
        }

        fn release_lease(
            &self,
            _lease_id: &LeaseId,
        ) -> impl std::future::Future<Output = Result<Option<String>, DomainError>> + Send {
            ready(Err(DomainError::InfrastructureError(
                "not used in ui tests".to_string(),
            )))
        }

        fn renew_lease(
            &self,
            _lease_id: &LeaseId,
            _extension_seconds: i64,
        ) -> impl std::future::Future<Output = Result<(), DomainError>> + Send {
            ready(Err(DomainError::InfrastructureError(
                "not used in ui tests".to_string(),
            )))
        }

        fn heartbeat_lease(
            &self,
            _lease_id: &LeaseId,
        ) -> impl std::future::Future<Output = Result<(), DomainError>> + Send {
            ready(Ok(()))
        }

        fn waitlist_resource(
            &self,
            _pool_type: String,
            _owner_id: String,
            _tenant_id: String,
            _priority: i32,
        ) -> impl std::future::Future<Output = Result<(), DomainError>> + Send {
            ready(Err(DomainError::InfrastructureError(
                "not used in ui tests".to_string(),
            )))
        }

        fn get_summary_stats(
            &self,
        ) -> impl std::future::Future<Output = Result<SummaryStats, DomainError>> + Send {
            ready(
                self.stats.clone().ok_or_else(|| {
                    DomainError::InfrastructureError("stats unavailable".to_string())
                }),
            )
        }

        fn get_recent_audit_logs(
            &self,
            _limit: i64,
        ) -> impl std::future::Future<Output = Result<Vec<AuditLogEntry>, DomainError>> + Send
        {
            ready(
                self.logs.clone().ok_or_else(|| {
                    DomainError::InfrastructureError("logs unavailable".to_string())
                }),
            )
        }

        fn fulfill_next_waitlist_entry(
            &self,
            _pool_type: String,
        ) -> impl std::future::Future<Output = Result<Option<Lease>, DomainError>> + Send {
            ready(Err(DomainError::InfrastructureError(
                "not used in ui tests".to_string(),
            )))
        }
    }

    fn build_state(repo: FakeRepository) -> AppState<FakeRepository> {
        let repo = Arc::new(repo);
        AppState {
            pool_service: Arc::new(PoolService::new(repo.clone())),
            resource_service: Arc::new(ResourceService::new(repo.clone())),
            allocation_service: Arc::new(AllocationService::new(repo)),
        }
    }

    async fn response_body(response: axum::response::Response) -> String {
        let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        String::from_utf8(bytes.to_vec()).unwrap()
    }

    #[tokio::test]
    async fn admin_dashboard_renders_stats_and_logs() {
        let state = build_state(FakeRepository {
            stats: Some(SummaryStats {
                active_leases: 7,
                total_resources: 21,
                waitlist_count: 3,
                healthy_resources: 18,
            }),
            logs: Some(vec![AuditLogEntry {
                id: 99,
                actor_id: Some("team-alpha".to_string()),
                action: Some("ALLOCATED".to_string()),
                resource_id: None,
                created_at: Utc::now(),
            }]),
        });

        let response = admin_dashboard(State(state)).await.into_response();
        let body = response_body(response).await;

        assert!(body.contains("System Overview"));
        assert!(body.contains("Recent Activity"));
        assert!(body.contains("team-alpha"));
        assert!(body.contains("ALLOCATED"));
        assert!(body.contains(">7<"));
        assert!(body.contains(">18<"));
    }

    #[tokio::test]
    async fn admin_dashboard_falls_back_to_zero_stats_on_repo_error() {
        let state = build_state(FakeRepository {
            stats: None,
            logs: Some(vec![]),
        });

        let response = admin_dashboard(State(state)).await.into_response();
        assert_eq!(response.status(), StatusCode::OK);

        let body = response_body(response).await;
        assert!(body.contains("System Overview"));
        assert!(body.contains(">0<"));
    }

    #[tokio::test]
    async fn audit_log_stream_renders_fragment_with_defaults() {
        let state = build_state(FakeRepository {
            stats: Some(SummaryStats {
                active_leases: 0,
                total_resources: 0,
                waitlist_count: 0,
                healthy_resources: 0,
            }),
            logs: Some(vec![AuditLogEntry {
                id: 5,
                actor_id: None,
                action: None,
                resource_id: None,
                created_at: Utc::now(),
            }]),
        });

        let response = audit_log_stream(State(state)).await.into_response();
        assert_eq!(response.status(), StatusCode::OK);

        let body = response_body(response).await;
        assert!(body.contains("OP"));
        assert!(body.contains("-"));
        assert!(body.contains("ID: 5"));
    }
}
