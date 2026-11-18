//! `web` binary: configuration, logging and graceful shutdown around [`web::ssr::router`].
//!
//! Environment:
//! * `WEB_BIND` – listen address (default `127.0.0.1:8080`).
//! * `WEB_UI_GALLERY` – `1`/`true` serves the component gallery at `/_ui` (never in production).
//! * `WEB_PUBLIC_BASE_URL` – public origin for canonical/`hreflang` links (unset until D-4).
//! * `RUST_LOG` – tracing filter (default `info`).

#![forbid(unsafe_code)]

use std::net::SocketAddr;

use anyhow::Context;
use tracing_subscriber::EnvFilter;
use web::ssr::{self, SsrConfig};

const DEFAULT_BIND: &str = "127.0.0.1:8080";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let lookup = |name: &str| std::env::var(name).ok();
    let bind: SocketAddr = lookup("WEB_BIND")
        .unwrap_or_else(|| DEFAULT_BIND.to_owned())
        .parse()
        .context("WEB_BIND must be a socket address such as 127.0.0.1:8080")?;
    let config = SsrConfig::from_lookup(lookup).context("invalid web configuration")?;

    let listener = tokio::net::TcpListener::bind(bind)
        .await
        .with_context(|| format!("cannot listen on {bind}"))?;
    tracing::info!(%bind, ui_gallery = config.ui_gallery, "web listening");

    axum::serve(listener, ssr::router(config))
        .with_graceful_shutdown(shutdown_signal())
        .await
        .context("server error")?;
    tracing::info!("web stopped");
    Ok(())
}

/// Resolves on Ctrl-C or SIGTERM (what orchestrators send), letting in-flight requests finish.
async fn shutdown_signal() {
    let ctrl_c = async {
        if let Err(error) = tokio::signal::ctrl_c().await {
            tracing::warn!(%error, "cannot listen for Ctrl-C");
            std::future::pending::<()>().await;
        }
    };
    #[cfg(unix)]
    let terminate = async {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut stream) => {
                stream.recv().await;
            }
            Err(error) => {
                tracing::warn!(%error, "cannot listen for SIGTERM");
                std::future::pending::<()>().await;
            }
        }
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        () = ctrl_c => {},
        () = terminate => {},
    }
}
