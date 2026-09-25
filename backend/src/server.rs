use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    response::{
        IntoResponse,
        sse::{Event, KeepAlive, Sse},
    },
    routing::{get, post},
};
use nle_cloud_sdk::client::NleCloudClient;
use serde::Deserialize;
use std::sync::Arc;
use tokio::sync::broadcast;
use tokio_stream::StreamExt as _;
use tokio_stream::wrappers::BroadcastStream;
use tower_http::cors::{Any, CorsLayer};

use crate::cli::ServeArgs;
use crate::project;
use crate::state::ProjectManager;

#[derive(Clone)]
pub struct AppState {
    pub tx: broadcast::Sender<String>,
    pub client: Arc<NleCloudClient>,
    pub project: Arc<ProjectManager>,
}

#[derive(Debug, Deserialize)]
pub struct ControlRequest {
    pub device_id: Option<i32>,
    pub tag: String,
    pub value: serde_json::Value,
}

/// Server-Sent Events (SSE) stream endpoint for reactive frontend updates (`EventSource`).
/// Streams an initial `"init"` event with the current project/device snapshot,
/// followed by `"update"` events whenever polling detects state changes or commands execute.
async fn sse_handler(
    State(state): State<AppState>,
) -> Sse<impl tokio_stream::Stream<Item = Result<Event, std::convert::Infallible>>> {
    let rx = state.tx.subscribe();

    // 1. Initial event containing the full snapshot immediately on connection
    let initial_snapshot = state.project.snapshot().await;
    let initial_json = serde_json::to_string(&initial_snapshot).unwrap_or_default();
    let initial_event = Ok(Event::default().event("init").data(initial_json));

    // 2. Chained with broadcast updates
    let stream = tokio_stream::once(initial_event).chain(
        BroadcastStream::new(rx).filter_map(|msg| match msg {
            Ok(data) => Some(Ok(Event::default().event("update").data(data))),
            Err(e) => {
                tracing::debug!("SSE broadcast stream skipped lagged message: {e}");
                None
            }
        }),
    );

    Sse::new(stream).keep_alive(KeepAlive::default())
}

/// Returns a JSON list of all device snapshots.
async fn get_devices_handler(State(state): State<AppState>) -> impl IntoResponse {
    Json(state.project.get_device_snapshots().await)
}

/// Returns a JSON snapshot of a specific device by ID.
async fn get_device_handler(
    State(state): State<AppState>,
    Path(device_id): Path<i32>,
) -> impl IntoResponse {
    match state.project.get_device_snapshot(device_id).await {
        Some(snapshot) => (StatusCode::OK, Json(serde_json::to_value(snapshot).unwrap())).into_response(),
        None => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "error": format!("Device ID {device_id} not found in this project")
            })),
        )
            .into_response(),
    }
}

/// Returns the full project snapshot including metadata and all devices.
async fn get_project_handler(State(state): State<AppState>) -> impl IntoResponse {
    Json(state.project.snapshot().await)
}

/// Dispatches an actuator command to a device.
/// If the device is offline, returns HTTP 409 Conflict.
/// On success, immediately broadcasts an SSE update to all connected clients.
async fn control_handler(
    State(state): State<AppState>,
    Json(payload): Json<ControlRequest>,
) -> impl IntoResponse {
    match state
        .project
        .control_actuator(&state.client, payload.device_id, &payload.tag, &payload.value)
        .await
    {
        Ok((used_id, confirmed_val)) => {
            // Immediately broadcast updated snapshot via SSE
            let snapshot = state.project.snapshot().await;
            if let Ok(update_json) = serde_json::to_string(&snapshot) {
                let _ = state.tx.send(update_json);
            }

            (
                StatusCode::OK,
                Json(serde_json::json!({
                    "success": true,
                    "device_id": used_id,
                    "tag": payload.tag,
                    "value": confirmed_val,
                })),
            )
                .into_response()
        }
        Err(err) => {
            let err_str = err.to_string();
            let status = if err_str.contains("offline") {
                StatusCode::CONFLICT
            } else {
                StatusCode::BAD_REQUEST
            };

            (
                status,
                Json(serde_json::json!({
                    "success": false,
                    "error": err_str,
                })),
            )
                .into_response()
        }
    }
}

async fn health_handler() -> impl IntoResponse {
    Json(serde_json::json!({ "status": "ok" }))
}

pub async fn run(args: ServeArgs) -> anyhow::Result<()> {
    let project_name = args.project.resolved_name();

    // 1. Authenticate with NLECloud
    let client = project::get_client(&args.project).await?;
    let client = Arc::new(client);

    // 2. Discover and instantiate project & devices in memory
    let project_manager = ProjectManager::init_from_cloud(
        &client,
        &project_name,
        args.project.device_namespace.as_deref(),
    )
    .await?;
    let project_manager = Arc::new(project_manager);

    // 3. Broadcast channel for real-time SSE stream events
    let (tx, _rx) = broadcast::channel(100);
    let state = AppState {
        tx: tx.clone(),
        client: client.clone(),
        project: project_manager.clone(),
    };

    // 4. Spawn background polling task
    let poll_state = state.clone();
    let poll_interval = tokio::time::Duration::from_millis(args.poll_interval_ms);
    tokio::spawn(async move {
        tracing::info!(
            "Background polling loop started with interval {:?} for project '{}'",
            poll_interval,
            poll_state.project.name
        );
        let mut ticker = tokio::time::interval(poll_interval);
        // First tick returns immediately, consume it
        ticker.tick().await;

        loop {
            ticker.tick().await;
            if poll_state.project.poll_all(&poll_state.client).await {
                let snapshot = poll_state.project.snapshot().await;
                if let Ok(update_json) = serde_json::to_string(&snapshot) {
                    let _ = poll_state.tx.send(update_json);
                }
            }
        }
    });

    // 5. Setup HTTP and Server-Sent Events (SSE) routes with CORS
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .route("/events", get(sse_handler))
        .route("/api/events", get(sse_handler))
        .route("/api/devices", get(get_devices_handler))
        .route("/api/devices/{device_id}", get(get_device_handler))
        .route("/api/project", get(get_project_handler))
        .route("/api/control", post(control_handler))
        .route("/health", get(health_handler))
        .route("/api/health", get(health_handler))
        .layer(cors)
        .with_state(state);

    let addr = format!("{}:{}", args.host, args.port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;

    tracing::info!("Server listening on http://{addr} (SSE at http://{addr}/api/events)");

    axum::serve(listener, app).await?;
    Ok(())
}
