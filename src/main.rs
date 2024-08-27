use std::{net::{IpAddr, Ipv6Addr, SocketAddr, SocketAddrV6}, path::PathBuf};

use axum::{extract::Host, handler::HandlerWithoutStateExt, http::{StatusCode, Uri}, response::Redirect, BoxError, Router};
use axum_server::tls_rustls::RustlsConfig;
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

#[derive(Clone, Copy)]
struct Ports {
    http: u16,
    https: u16,
}

async fn redirect_http_to_https(ports: Ports) {
    fn make_https(host: String, uri: Uri, ports: Ports) -> Result<Uri, BoxError> {
        let mut parts = uri.into_parts();

        parts.scheme = Some(axum::http::uri::Scheme::HTTPS);

        if parts.path_and_query.is_none() {
            parts.path_and_query = Some("/".parse().unwrap());
        }

        let https_host = host.replace(&ports.http.to_string(), &ports.https.to_string());
        parts.authority = Some(https_host.parse()?);

        Ok(Uri::from_parts(parts)?)
    }

    let redirect = move |Host(host): Host, uri: Uri| async move {
        match make_https(host, uri, ports) {
            Ok(uri) => Ok(Redirect::permanent(&uri.to_string())),
            Err(error) => {
                tracing::warn!(%error, "failed to convert URI to HTTPS");
                Err(StatusCode::BAD_REQUEST)
            }
        }
    };

    let addr = SocketAddr::new(IpAddr::from(Ipv6Addr::UNSPECIFIED), ports.http);
    let listener = TcpListener::bind(addr).await.unwrap();
    tracing::debug!("(http->https) listening on {}", addr);

    axum::serve(listener, redirect.into_make_service())
        .await
        .unwrap();
}

#[tokio::main]
async fn main() {
    tracing_init();

    let ports = Ports {
        http: std::env::var("PORT").unwrap_or("8080".to_string()).parse().unwrap(),
        https: std::env::var("SPORT").unwrap_or("8443".to_string()).parse().unwrap(),
    };

    tokio::spawn(redirect_http_to_https(ports));

    let tls_config = RustlsConfig::from_pem_file(
        PathBuf::from("./crt")
            .join("race_hked_live.crt"), 
        PathBuf::from("./crt")
            .join("race_hked_live.key")
    ).await.unwrap();
    
    // ipv6 address
    let addr = SocketAddr::new(IpAddr::from(Ipv6Addr::UNSPECIFIED), ports.https);
    
    debug!("(main) listening on {}", addr);

    let app = router();

    axum_server::bind_rustls(addr, tls_config)
        .serve(app.into_make_service())
        .await
        .unwrap();
}
