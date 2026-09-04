use axum::{
    Router,
    extract::{
        State,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    response::IntoResponse,
    routing::get,
};
use tokio::sync::broadcast;
use tower_http::cors::{Any, CorsLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Clone)]
struct AppState {
    tx: broadcast::Sender<String>,
}

async fn ws_handler(ws: WebSocketUpgrade, State(state): State<AppState>) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

async fn handle_socket(mut socket: WebSocket, state: AppState) {
    // Subscribe to the broadcast channel for this specific client
    let mut rx = state.tx.subscribe();

    loop {
        tokio::select! {
            // Branch 1: Receive a message from the broadcast channel and send it to the client
            result = rx.recv() => {
                match result {
                    Ok(msg) => {
                        if socket.send(Message::Text(msg.into())).await.is_err() {
                            // Client disconnected or send failed
                            break;
                        }
                    }
                    Err(_) => {
                        // The broadcast channel was closed (shouldn't happen here) or we lagged.
                        break;
                    }
                }
            }
            // Branch 2: Receive a message from the client
            result = socket.recv() => {
                match result {
                    Some(Ok(Message::Text(text))) => {
                        // Broadcast the received message to all connected clients
                        let _ = state.tx.send(text.to_string());
                    }
                    Some(Ok(Message::Close(_))) | None => {
                        // Client gracefully closed the connection or the stream ended
                        break;
                    }
                    _ => {
                        // Ignore Ping, Pong, Binary messages, or minor errors for this simple example
                    }
                }
            }
        }
    }

    tracing::info!("Client disconnected");
}

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,axum=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Create a broadcast channel with a capacity of 100 messages
    let (tx, _rx) = broadcast::channel(100);
    let state = AppState { tx };

    // Configure CORS to allow all origins, methods, and headers
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .route("/ws", get(ws_handler))
        .layer(cors)
        .with_state(state);

    // Bind to 0.0.0.0 to expose to the local network
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();

    tracing::info!("WebSocket server listening on ws://0.0.0.0:3000/ws");

    axum::serve(listener, app).await.unwrap();
}
