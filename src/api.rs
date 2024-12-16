use axum::{
    routing::get,
    Router,
    extract::{State, Path, Query},
    response::{IntoResponse, Json, Html},
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::net::SocketAddr;
use crate::storage::Storage;
use crate::change_logger::ChangeLogger;
use tokio::signal;
use std::time::Instant;

pub struct Api {
    storage: Arc<Storage>,
    change_logger: Arc<ChangeLogger>,
    start_time: Instant,
}

#[derive(Clone)]
struct AppState {
    api: Arc<Api>,
}

#[derive(Deserialize)]
struct PaginationParams {
    offset: Option<usize>,
    max_results: Option<usize>,
}

#[derive(Serialize)]
struct ApiResponse<T> {
    data: T,
    has_more: bool,
    offset: usize,
    max_results: usize,
}

#[derive(Serialize)]
struct ChangeSummary {
    revision: String,
    timestamp: u64,
    service: String,
    summary: SummaryDetails,
}

#[derive(Serialize)]
struct SummaryDetails {
    additions: usize,
    modifications: usize,
    deletions: usize,
    tags: Vec<String>,
}

#[derive(Serialize)]
struct ChangeDetails {
    additions: Vec<ChangeItem>,
    modifications: Vec<ChangeItem>,
    deletions: Vec<ChangeItem>,
}

#[derive(Serialize)]
struct ChangeItem {
    path: String,
    value: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    old_value: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    new_value: Option<serde_json::Value>,
}

#[derive(Serialize)]
struct DiffFormatResponse {
    service: String,
    timestamp: String,
    changes: Vec<DiffEntry>,
}

#[derive(Serialize)]
struct DiffEntry {
    change_type: String,  // "+" for addition, "-" for deletion, "M" for modification
    path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    old_value: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    new_value: Option<serde_json::Value>,
}

impl Api {
    pub fn new(storage: Storage, change_logger: ChangeLogger) -> Self {
        Api {
            storage: Arc::new(storage),
            change_logger: Arc::new(change_logger),
            start_time: Instant::now(),
        }
    }

    pub async fn run(self, addr: SocketAddr) {
        let app_state = AppState {
            api: Arc::new(self),
        };

        let app = Router::new()
            .route("/", get(root))
            .route("/api/status", get(status))
            .route("/api/changes", get(all_changes))
            .route("/api/changes/:service", get(service_changes))
            .route("/api/changes/:service/:timestamp", get(specific_change))
            .route("/api/changes/:service/:timestamp/diff", get(diff_format_change))
            .with_state(app_state);

        println!("API server listening on {}", addr);

        let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
        axum::serve(listener, app)
            .with_graceful_shutdown(shutdown_signal())
            .await
            .unwrap();
    }
}

async fn root() -> impl IntoResponse {
    Html(r#"
    <link rel="stylesheet" href="//cdn.jsdelivr.net/gh/KrauseFx/markdown-to-html-github-style@master/style.css">
    <h1 id="googlediscoverydocumenttracker">Google Discovery Document Tracker API</h1>
    <h3 id="getapistatus"><code>GET /api/status</code></h3>
    <ul>
    <li>What is returned: JSON object containing uptime information and a list of tracked services with their change counts.</li>
    </ul>
    <h3 id="getapichanges"><code>GET /api/changes</code></h3>
    <ul>
    <li>What is returned: JSON object containing a list of changes for all tracked services, with timestamps for each change.</li>
    </ul>
    <h3 id="getapichangesservice"><code>GET /api/changes/:service</code></h3>
    <ul>
    <li>What is returned: JSON array of timestamps for all changes detected for the specified service.</li>
    </ul>
    <h3 id="getapichangesservicedatetime"><code>GET /api/changes/:service/:datetime</code></h3>
    <ul>
    <li>What is returned: JSON object containing details of the changes made to the specified service at the given datetime.</li>
    <li>The datetime should be in unix format.</li>
    </ul>
    "#)
}

async fn status(State(state): State<AppState>) -> impl IntoResponse {
    let uptime = state.api.start_time.elapsed().as_secs();
    let services = state.api.storage.retrieve_all().unwrap();
    let service_names: Vec<String> = services.keys().cloned().collect();

    Json(serde_json::json!({
        "uptime": uptime,
        "services": service_names,
    }))
}

async fn diff_format_change(
    State(state): State<AppState>,
    Path((service, timestamp)): Path<(String, String)>,
) -> impl IntoResponse {
    let change = state.api.change_logger.get_specific_change(&service, &timestamp).unwrap();
    
    let mut diff_entries = Vec::new();

    // Process additions
    for addition in change.additions {
        diff_entries.push(DiffEntry {
            change_type: "+".to_string(),
            path: addition.path,
            old_value: None,
            new_value: addition.value,
        });
    }

    // Process deletions
    for deletion in change.deletions {
        diff_entries.push(DiffEntry {
            change_type: "-".to_string(),
            path: deletion.path,
            old_value: deletion.old_value,
            new_value: None,
        });
    }

    // Process modifications
    for modification in change.modifications {
        diff_entries.push(DiffEntry {
            change_type: "M".to_string(),
            path: modification.path,
            old_value: modification.old_value,
            new_value: modification.new_value,
        });
    }

    // Sort entries by change type (+ first, then -, then M) and then by path
    diff_entries.sort_by(|a, b| {
        // Custom ordering for change types
        let type_order = |t: &str| match t {
            "+" => 0,
            "-" => 1,
            "M" => 2,
            _ => 3,
        };
        
        // Compare change types first
        let type_comparison = type_order(&a.change_type).cmp(&type_order(&b.change_type));
        
        // If change types are equal, compare paths
        if type_comparison == std::cmp::Ordering::Equal {
            a.path.cmp(&b.path)
        } else {
            type_comparison
        }
    });

    let response = DiffFormatResponse {
        service,
        timestamp,
        changes: diff_entries,
    };

    // Create formatted JSON response
    let json_str = serde_json::to_string_pretty(&response).unwrap();
    
    // Return with proper content type
    (
        [(axum::http::header::CONTENT_TYPE, "application/json")],
        json_str
    )
}

async fn all_changes(
    State(state): State<AppState>,
    Query(params): Query<PaginationParams>,
) -> impl IntoResponse {
    let (offset, max_results) = get_pagination_params(params);
    let all_changes = state.api.change_logger.get_all_changes(offset, max_results + 1).unwrap();
    let has_more = all_changes.len() > max_results;
    let changes = all_changes.into_iter().take(max_results)
        .map(|change| ChangeSummary {
            revision: change.revision,
            timestamp: change.timestamp,
            service: change.service,
            summary: SummaryDetails {
                additions: change.summary.additions,
                modifications: change.summary.modifications,
                deletions: change.summary.deletions,
                tags: change.summary.tags,
            },
        })
        .collect::<Vec<_>>();
    
    Json(ApiResponse {
        data: changes,
        has_more,
        offset,
        max_results,
    })
}

async fn service_changes(
    State(state): State<AppState>,
    Path(service): Path<String>,
    Query(params): Query<PaginationParams>,
) -> impl IntoResponse {
    let (offset, max_results) = get_pagination_params(params);
    let changes = state.api.change_logger.get_changes_for_service(&service, offset, max_results + 1).unwrap();
    let has_more = changes.len() > max_results;
    let summaries = changes.into_iter().take(max_results)
        .map(|change| ChangeSummary {
            revision: change.revision,
            timestamp: change.timestamp,
            service: change.service,
            summary: SummaryDetails {
                additions: change.summary.additions,
                modifications: change.summary.modifications,
                deletions: change.summary.deletions,
                tags: change.summary.tags,
            },
        })
        .collect::<Vec<_>>();
    
    Json(ApiResponse {
        data: summaries,
        has_more,
        offset,
        max_results,
    })
}

async fn specific_change(
    State(state): State<AppState>,
    Path((service, timestamp)): Path<(String, String)>,
) -> impl IntoResponse {
    let change = state.api.change_logger.get_specific_change(&service, &timestamp).unwrap();
    
    let details = ChangeDetails {
        additions: change.additions.into_iter().map(|c| ChangeItem {
            path: c.path,
            value: c.value,
            old_value: c.old_value,
            new_value: c.new_value,
        }).collect(),
        modifications: change.modifications.into_iter().map(|c| ChangeItem {
            path: c.path,
            value: c.value,
            old_value: c.old_value,
            new_value: c.new_value,
        }).collect(),
        deletions: change.deletions.into_iter().map(|c| ChangeItem {
            path: c.path,
            value: c.value,
            old_value: c.old_value,
            new_value: c.new_value,
        }).collect(),
    };
    
    let json_str = serde_json::to_string_pretty(&details).unwrap();
    // Return with proper content type
    (
        [(axum::http::header::CONTENT_TYPE, "application/json")],
        json_str
    )
}

fn get_pagination_params(params: PaginationParams) -> (usize, usize) {
    let offset = params.offset.unwrap_or(0);
    let max_results = params.max_results.unwrap_or(50).min(50);
    (offset, max_results)
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    println!("signal received, starting graceful shutdown");
}