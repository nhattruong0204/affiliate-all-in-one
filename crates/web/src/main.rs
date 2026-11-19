//! `web` binary: configuration, logging and graceful shutdown around [`web::ssr::router`].
//!
//! Environment (shared settings are documented in `app::config`, operations in
//! `docs/OBSERVABILITY.md`):
//! * `APP_ENV`, `RUST_LOG`, `LOG_FORMAT`, `DATABASE_URL[_FILE]`, `READINESS_TIMEOUT_MS`.
//! * `WEB_BIND` – public listener (default `127.0.0.1:8080`).
//! * `METRICS_BIND` – internal listener serving only `/metrics` (default `127.0.0.1:9100`);
//!   never publish this port.
//! * `WEB_UI_GALLERY` – `1`/`true` serves the component gallery at `/_ui` (refused in production).
//! * `WEB_PUBLIC_BASE_URL` – public origin for canonical/`hreflang` links (unset until D-4).

#![forbid(unsafe_code)]

use std::sync::Arc;

use anyhow::Context;
use app::config::{AppEnv, Config};
use app::health::Readiness;
use app::metrics::Metrics;
use app::telemetry;
use axum::Router;
use tokio::net::TcpListener;
use tokio::sync::watch;
use web::health::metrics_router;
use web::ssr::{self, SsrConfig};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Configuration errors print to stderr before logging exists, all at once.
    let config = Config::from_env().context("invalid startup configuration")?;
    telemetry::init(&config).context("cannot initialise logging")?;

    let ssr_config = SsrConfig::from_lookup(|name| std::env::var(name).ok())
        .map_err(anyhow::Error::msg)
        .context("invalid web configuration")?;
    if config.env == AppEnv::Production && ssr_config.ui_gallery {
        anyhow::bail!("WEB_UI_GALLERY must not be enabled when APP_ENV=production");
    }

    let metrics = Arc::new(Metrics::new());
    let readiness = Arc::new(Readiness::from_config(&config));

    let public = TcpListener::bind(config.web_bind)
        .await
        .with_context(|| format!("cannot listen on {}", config.web_bind))?;
    let internal = TcpListener::bind(config.metrics_bind)
        .await
        .with_context(|| format!("cannot listen on {}", config.metrics_bind))?;
    tracing::info!(
        web = %config.web_bind,
        metrics = %config.metrics_bind,
        env = ?config.env,
        ui_gallery = ssr_config.ui_gallery,
        database_configured = config.database.is_some(),
        "web listening"
    );

    let (stop_tx, stop_rx) = watch::channel(false);
    tokio::spawn(async move {
        shutdown_signal().await;
        let _ = stop_tx.send(true);
    });

    tokio::try_join!(
        serve(
            public,
            ssr::router(ssr_config, readiness, metrics.clone()),
            stop_rx.clone()
        ),
        serve(internal, metrics_router(metrics), stop_rx),
    )
    .context("server error")?;
    tracing::info!("web stopped");
    Ok(())
}

/// Serves `router` until `stop` flips to `true`, letting in-flight requests finish.
async fn serve(
    listener: TcpListener,
    router: Router,
    mut stop: watch::Receiver<bool>,
) -> std::io::Result<()> {
    axum::serve(listener, router)
        .with_graceful_shutdown(async move {
            let _ = stop.wait_for(|stopped| *stopped).await;
        })
        .await
}

/// Resolves on Ctrl-C or SIGTERM (what orchestrators send).
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
