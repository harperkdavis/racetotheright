use std::net::{IpAddr, Ipv6Addr, SocketAddr, SocketAddrV6};

use axum::Router;
use tokio::net::TcpListener;
use tower_http::{services::ServeDir, trace::TraceLayer};
use tracing::debug;

fn router() -> Router {
    Router::new()
        .nest_service("/", ServeDir::new("client").append_index_html_on_directories(true))
}

fn tracing_init() {
    use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                format!("{}=debug,tower_http=debug", env!("CARGO_CRATE_NAME")).into()
            })
        )
        .with(tracing_subscriber::fmt::layer())
        .init();
}

#[tokio::main]
async fn main() {
    tracing_init();
    
    // ipv6 address
    let addr = SocketAddr::new(IpAddr::from(Ipv6Addr::LOCALHOST), 8080);
    let listener = TcpListener::bind(addr).await.unwrap();
    
    debug!("listening on {}", addr);

    let app = router();

    axum::serve(listener, app.layer(TraceLayer::new_for_http()))
        .await
        .unwrap();
}
