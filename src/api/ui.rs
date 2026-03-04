use axum::{
    response::{Html, IntoResponse},
    extract::State,
    http::StatusCode,
};
use askama::Template;
use crate::api::routes::AppState;
use crate::domain::repository::{SummaryStats, AuditLogEntry};

#[derive(Template)]
#[template(path = "admin.html")]
pub struct AdminTemplate {
    pub stats: SummaryStats,
}

pub async fn admin_dashboard(
    State(state): State<AppState>,
) -> impl IntoResponse {
    let stats = match state.allocation_service.get_stats().await {
        Ok(s) => s,
        Err(_) => SummaryStats {
            active_leases: 0,
            total_resources: 0,
            waitlist_count: 0,
            healthy_resources: 0,
        }
    };

    let template = AdminTemplate { stats };
    match template.render() {
        Ok(html) => Html(html).into_response(),
        Err(e) => {
            tracing::error!("Template render error: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "Template error").into_response()
        }
    }
}

#[derive(Template)]
#[template(source = r#"
    {% for log in logs %}
    <div class="flex items-center space-x-3 py-2 border-b border-slate-800/50 last:border-0 text-sm">
        <span class="text-slate-500 font-mono text-[10px] w-24">{{ log.created_at.format("%H:%M:%S") }}</span>
        <span class="px-2 py-0.5 rounded text-[10px] font-bold uppercase bg-blue-500/10 text-blue-500">
            {{ log.action.as_deref().unwrap_or("OP") }}
        </span>
        <span class="text-slate-300 truncate max-w-[120px]">{{ log.actor_id.as_deref().unwrap_or("-") }}</span>
        <span class="text-slate-500 text-[10px] truncate flex-1">ID: {{ log.id }}</span>
    </div>
    {% endfor %}
"#, ext = "html")]
pub struct AuditLogTemplate {
    pub logs: Vec<AuditLogEntry>,
}

pub async fn audit_log_stream(
    State(state): State<AppState>,
) -> impl IntoResponse {
    let logs = state.allocation_service.get_recent_logs(15).await.unwrap_or_default();
    let template = AuditLogTemplate { logs };
    match template.render() {
        Ok(html) => Html(html).into_response(),
        Err(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Log error").into_response()
    }
}
