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
//!
//! `web healthcheck` probes the running server's `/healthz` and exits non-zero on failure; it is
//! what the compose container healthcheck runs (the image has no shell or `curl`).

#![forbid(unsafe_code)]

use std::net::SocketAddr;
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
use web::middleware::Hardening;
use web::ssr::{self, SsrConfig};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Container health probe: no configuration, no logging, just an exit status.
    if std::env::args().nth(1).as_deref() == Some("healthcheck") {
        return web::healthcheck::run().map_err(anyhow::Error::msg);
    }

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
        turnstile_enabled = config.turnstile.is_some(),
        trust_forwarded_for = config.trust_forwarded_for,
        "web listening"
    );
    if config.env == AppEnv::Production {
        if config.turnstile.is_none() {
            tracing::warn!(
                "CAPTCHA is not configured: anonymous endpoints are protected by rate limits only"
            );
        }
        if !config.trust_forwarded_for {
            tracing::warn!(
                "TRUST_FORWARDED_FOR is off: behind a reverse proxy every client shares the proxy's rate-limit bucket"
            );
        }
    }

    let (stop_tx, stop_rx) = watch::channel(false);
    tokio::spawn(async move {
        shutdown_signal().await;
        let _ = stop_tx.send(true);
    });

    tokio::try_join!(
        serve(
            public,
            ssr::router(
                ssr_config,
                readiness,
                metrics.clone(),
                Hardening::from_config(&config)
            ),
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
    // Connect info gives the rate limiter the TCP peer address.
    axum::serve(
        listener,
        router.into_make_service_with_connect_info::<SocketAddr>(),
    )
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
