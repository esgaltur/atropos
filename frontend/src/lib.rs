use atropos_contracts::{AuditLogItem, DashboardStats};
use leptos::*;

const HTMX_URL: &str = "https://unpkg.com/htmx.org@1.9.10";
const TAILWIND_CDN_URL: &str = "https://cdn.tailwindcss.com";
const FONT_AWESOME_URL: &str =
    "https://cdnjs.cloudflare.com/ajax/libs/font-awesome/6.0.0/css/all.min.css";

pub fn render_dashboard(stats: DashboardStats, logs: Vec<AuditLogItem>) -> String {
    leptos::ssr::render_to_string(move || {
        view! {
            <!DOCTYPE html>
            <html lang="en">
                <head>
                    <meta charset="UTF-8" />
                    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
                    <title>"Atropos | Resource Orchestration"</title>
                    <DashboardAssets />
                </head>
                <body class="min-h-screen">
                    <TopNav />

                    <main class="max-w-7xl mx-auto px-6 pb-12">
                        <DashboardHeader />
                        <StatsGrid stats=stats />
                        <DashboardPanels logs=logs />
                    </main>

                    <DashboardFooter />
                </body>
            </html>
        }
    })
    .to_string()
}

#[component]
fn DashboardAssets() -> impl IntoView {
    view! {
        <script src=HTMX_URL></script>
        <script src=TAILWIND_CDN_URL></script>
        <link href=FONT_AWESOME_URL rel="stylesheet" />
        <style>
            "@import url('https://fonts.googleapis.com/css2?family=Inter:wght@300;400;600;700&display=swap');"
            "body { font-family: 'Inter', sans-serif; background-color: #0f172a; color: #f8fafc; }"
            ".glass { background: rgba(30, 41, 59, 0.7); backdrop-filter: blur(12px); border: 1px solid rgba(255,255,255,0.1); }"
            ".stat-card { transition: transform 0.2s; }"
            ".stat-card:hover { transform: translateY(-5px); }"
        </style>
    }
}

#[component]
fn TopNav() -> impl IntoView {
    view! {
        <nav class="glass sticky top-0 z-50 px-6 py-4 flex justify-between items-center mb-8">
            <div class="flex items-center space-x-3">
                <div class="bg-blue-600 p-2 rounded-lg">
                    <i class="fas fa-microchip text-xl"></i>
                </div>
                <span class="text-xl font-bold tracking-tight">"ATROPOS "<span class="text-blue-500 font-normal">"ELITE"</span></span>
            </div>
            <div class="flex space-x-6 text-sm font-medium text-slate-400">
                <a href="#" class="hover:text-white transition">"Dashboard"</a>
                <a href="/metrics" target="_blank" class="hover:text-white transition">"Metrics"</a>
                <a href="/openapi.yaml" target="_blank" class="hover:text-white transition">"API Docs"</a>
            </div>
        </nav>
    }
}

#[component]
fn DashboardHeader() -> impl IntoView {
    view! {
        <header class="mb-10">
            <h1 class="text-3xl font-bold mb-2">"System Overview"</h1>
            <p class="text-slate-400">"Real-time resource orchestration and lease governance."</p>
        </header>
    }
}

#[component]
fn StatsGrid(stats: DashboardStats) -> impl IntoView {
    view! {
        <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6 mb-10">
            <StatCard icon_class="fas fa-key text-xl" icon_bg="bg-blue-500/20" icon_text="text-blue-500" label="Active" value=stats.active_leases description="Total Active Leases" />
            <StatCard icon_class="fas fa-check-circle text-xl" icon_bg="bg-emerald-500/20" icon_text="text-emerald-500" label="Healthy" value=stats.healthy_resources description="Total Healthy Resources" />
            <StatCard icon_class="fas fa-hourglass-half text-xl" icon_bg="bg-amber-500/20" icon_text="text-amber-500" label="Waiting" value=stats.waitlist_count description="Requests on Waitlist" />
            <StatCard icon_class="fas fa-server text-xl" icon_bg="bg-indigo-500/20" icon_text="text-indigo-500" label="Total" value=stats.total_resources description="Managed Resources" />
        </div>
    }
}

#[component]
fn DashboardPanels(logs: Vec<AuditLogItem>) -> impl IntoView {
    view! {
        <div class="grid grid-cols-1 lg:grid-cols-3 gap-8">
            <HealthPanel />
            <RecentActivityPanel logs=logs />
        </div>
    }
}

#[component]
fn HealthPanel() -> impl IntoView {
    view! {
        <div class="lg:col-span-2 glass rounded-2xl p-8 flex flex-col">
            <div class="flex justify-between items-center mb-6">
                <h3 class="text-xl font-bold">"System Health Monitor"</h3>
                <button
                    hx-get="/health"
                    hx-target="#health-indicator"
                    class="bg-blue-600 hover:bg-blue-500 px-4 py-2 rounded-lg text-sm font-semibold transition flex items-center space-x-2"
                >
                    <i class="fas fa-sync-alt"></i>
                    <span>"Refresh Status"</span>
                </button>
            </div>

            <div class="bg-slate-900/50 rounded-xl p-10 flex flex-col items-center justify-center border border-slate-800 flex-1">
                <div id="health-indicator" class="text-center">
                    <div class="w-16 h-16 bg-slate-800 rounded-full flex items-center justify-center mb-4 mx-auto text-slate-600">
                        <i class="fas fa-heartbeat text-3xl"></i>
                    </div>
                    <p class="text-slate-500 font-medium">"Click refresh to check connectivity"</p>
                </div>
            </div>
        </div>
    }
}

#[component]
fn RecentActivityPanel(logs: Vec<AuditLogItem>) -> impl IntoView {
    view! {
        <div class="glass rounded-2xl p-8">
            <div class="flex items-center justify-between mb-6">
                <h3 class="text-xl font-bold">"Recent Activity"</h3>
                <span class="flex h-2 w-2 relative">
                    <span class="animate-ping absolute inline-flex h-full w-full rounded-full bg-emerald-400 opacity-75"></span>
                    <span class="relative inline-flex rounded-full h-2 w-2 bg-emerald-500"></span>
                </span>
            </div>
            <div class="space-y-1 overflow-y-auto max-h-[300px] pr-2 custom-scrollbar" hx-get="/admin/audit-log" hx-trigger="load, every 2s" hx-swap="innerHTML">
                <AuditLogEntries logs=logs />
            </div>
        </div>
    }
}

#[component]
fn AuditLogEntries(logs: Vec<AuditLogItem>) -> impl IntoView {
    view! {
        <For each=move || logs.clone() key=|log| log.id let:log>
            <AuditLogRow log=log />
        </For>
    }
}

#[component]
fn AuditLogRow(log: AuditLogItem) -> impl IntoView {
    view! {
        <div class="flex items-center space-x-3 py-2 border-b border-slate-800/50 last:border-0 text-sm">
            <span class="text-slate-500 font-mono text-[10px] w-24">{log.created_at}</span>
            <span class="px-2 py-0.5 rounded text-[10px] font-bold uppercase bg-blue-500/10 text-blue-500">{log.action}</span>
            <span class="text-slate-300 truncate max-w-[120px]">{log.actor_id}</span>
            <span class="text-slate-500 text-[10px] truncate flex-1">{"ID: "}{log.id}</span>
        </div>
    }
}

#[component]
fn DashboardFooter() -> impl IntoView {
    view! {
        <footer class="border-t border-slate-800 mt-12 py-8 text-center text-slate-500 text-sm">
            "© 2026 Atropos Resource Orchestration Platform. All threads accounted for."
        </footer>
    }
}

#[component]
fn StatCard(
    icon_class: &'static str,
    icon_bg: &'static str,
    icon_text: &'static str,
    label: &'static str,
    value: i64,
    description: &'static str,
) -> impl IntoView {
    view! {
        <div class="glass p-6 rounded-2xl stat-card">
            <div class="flex justify-between items-start mb-4">
                <div class=move || format!("{} p-3 rounded-xl {}", icon_bg, icon_text)>
                    <i class=icon_class></i>
                </div>
                <span class=move || format!("text-xs font-semibold {} uppercase tracking-wider", icon_text)>{label}</span>
            </div>
            <div class="text-3xl font-bold mb-1">{value}</div>
            <div class="text-sm text-slate-500">{description}</div>
        </div>
    }
}

pub fn render_audit_log(logs: Vec<AuditLogItem>) -> String {
    leptos::ssr::render_to_string(move || {
        view! {
            <AuditLogEntries logs=logs />
        }
    })
    .to_string()
}
