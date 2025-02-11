use crate::metrics::AppState;
use axum::{extract::State, routing::get, Router};
use prometheus::{Encoder, TextEncoder};
use std::net::SocketAddr;
use tokio::net::TcpListener;

async fn metrics_handler(State(state): State<AppState>) -> String {
    let encoder = TextEncoder::new();
    let mut buffer = Vec::new();
    encoder
        .encode(&state.registry.gather(), &mut buffer)
        .unwrap();
    String::from_utf8(buffer).unwrap()
}

pub async fn run_server(
    state: AppState,
    port: u16,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let app = Router::new()
        .route("/metrics", get(metrics_handler))
        .with_state(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    println!("Metrics server listening on http://{}/metrics", addr);

    let listener = TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
