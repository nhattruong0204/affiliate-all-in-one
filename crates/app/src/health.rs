//! Readiness: is this process able to serve traffic right now?
//!
//! `/healthz` only says the process is alive and never touches dependencies (so an orchestrator
//! does not restart a healthy process because the database blipped). `/readyz` runs the
//! registered [`ReadinessCheck`]s; any failure or timeout makes it 503 and takes the instance out
//! of rotation (Caddy then serves the maintenance page).
//!
//! The database check is a seam on purpose: `db` (AAO-004) owns sqlx, so until it lands the only
//! check is [`TcpProbe`], a TCP reachability test of the configured database host and port.
//! AAO-004 replaces it with `SELECT 1` plus "migrations current" behind the same trait.

use std::fmt;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

use serde_json::{Value, json};
use tokio::net::TcpStream;

use crate::config::Config;
use crate::error::FieldError;

/// Why a check failed. Logged server-side; never sent to clients.
#[derive(Debug, thiserror::Error)]
#[error("{0}")]
pub struct CheckError(pub String);

pub type CheckFuture<'a> = Pin<Box<dyn Future<Output = Result<(), CheckError>> + Send + 'a>>;

/// One dependency that must be healthy for the process to receive traffic.
pub trait ReadinessCheck: Send + Sync {
    /// Stable name used in the response and in metrics/alerts (`database`).
    fn name(&self) -> &'static str;
    fn check(&self) -> CheckFuture<'_>;
}

/// Succeeds when a TCP connection to `host:port` can be opened.
#[derive(Clone, Debug)]
pub struct TcpProbe {
    name: &'static str,
    host: String,
    port: u16,
}

impl TcpProbe {
    pub fn new(name: &'static str, host: impl Into<String>, port: u16) -> Self {
        Self {
            name,
            host: host.into(),
            port,
        }
    }
}

impl ReadinessCheck for TcpProbe {
    fn name(&self) -> &'static str {
        self.name
    }

    fn check(&self) -> CheckFuture<'_> {
        Box::pin(async move {
            TcpStream::connect((self.host.as_str(), self.port))
                .await
                .map(drop)
                .map_err(|error| CheckError(format!("cannot connect: {}", error.kind())))
        })
    }
}

/// Result of one check.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CheckOutcome {
    pub name: &'static str,
    pub ok: bool,
}

/// Result of a full readiness run.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReadinessReport {
    pub checks: Vec<CheckOutcome>,
}

impl ReadinessReport {
    pub fn is_ready(&self) -> bool {
        self.checks.iter().all(|check| check.ok)
    }

    /// The failing checks as problem+json `errors` entries (`database` / `unavailable`).
    pub fn failures(&self) -> Vec<FieldError> {
        self.checks
            .iter()
            .filter(|check| !check.ok)
            .map(|check| FieldError::new(check.name, "unavailable"))
            .collect()
    }

    /// Body of a `200` response.
    pub fn to_json(&self) -> Value {
        let checks: Vec<Value> = self
            .checks
            .iter()
            .map(|check| json!({ "name": check.name, "status": if check.ok { "ok" } else { "unavailable" } }))
            .collect();
        json!({ "status": if self.is_ready() { "ok" } else { "unavailable" }, "checks": checks })
    }
}

/// The set of checks and the per-check time budget.
pub struct Readiness {
    checks: Vec<Arc<dyn ReadinessCheck>>,
    timeout: Duration,
}

impl fmt::Debug for Readiness {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Readiness")
            .field(
                "checks",
                &self.checks.iter().map(|c| c.name()).collect::<Vec<_>>(),
            )
            .field("timeout", &self.timeout)
            .finish()
    }
}

impl Readiness {
    pub fn new(timeout: Duration) -> Self {
        Self {
            checks: Vec::new(),
            timeout,
        }
    }

    /// The checks implied by `config`: the database probe when a database is configured.
    pub fn from_config(config: &Config) -> Self {
        let mut readiness = Self::new(config.readiness_timeout);
        if let Some(database) = &config.database {
            readiness = readiness.with_check(Arc::new(TcpProbe::new(
                "database",
                database.host.clone(),
                database.port,
            )));
        }
        readiness
    }

    #[must_use]
    pub fn with_check(mut self, check: Arc<dyn ReadinessCheck>) -> Self {
        self.checks.push(check);
        self
    }

    /// Runs every check, each bounded by the timeout. A check that errors or times out is
    /// reported as failed and logged (with the reason) at `warn`.
    pub async fn run(&self) -> ReadinessReport {
        let mut outcomes = Vec::with_capacity(self.checks.len());
        for check in &self.checks {
            let name = check.name();
            let ok = match tokio::time::timeout(self.timeout, check.check()).await {
                Ok(Ok(())) => true,
                Ok(Err(error)) => {
                    tracing::warn!(check = name, %error, "readiness check failed");
                    false
                }
                Err(_) => {
                    tracing::warn!(check = name, "readiness check timed out");
                    false
                }
            };
            outcomes.push(CheckOutcome { name, ok });
        }
        ReadinessReport { checks: outcomes }
    }
}

#[cfg(test)]
mod tests {
    use tokio::net::TcpListener;

    use super::*;

    struct Fixed(&'static str, Result<(), &'static str>);

    impl ReadinessCheck for Fixed {
        fn name(&self) -> &'static str {
            self.0
        }
        fn check(&self) -> CheckFuture<'_> {
            Box::pin(async move { self.1.map_err(|m| CheckError(m.to_owned())) })
        }
    }

    struct Hangs;

    impl ReadinessCheck for Hangs {
        fn name(&self) -> &'static str {
            "slow"
        }
        fn check(&self) -> CheckFuture<'_> {
            Box::pin(std::future::pending())
        }
    }

    #[tokio::test]
    async fn no_checks_means_ready() {
        let report = Readiness::new(Duration::from_secs(1)).run().await;
        assert!(report.is_ready());
        assert_eq!(report.to_json()["status"], "ok");
    }

    #[tokio::test]
    async fn one_failing_check_fails_the_report_and_names_it() {
        let readiness = Readiness::new(Duration::from_secs(1))
            .with_check(Arc::new(Fixed("cache", Ok(()))))
            .with_check(Arc::new(Fixed("database", Err("refused"))));
        let report = readiness.run().await;
        assert!(!report.is_ready());
        assert_eq!(
            report.failures(),
            vec![FieldError::new("database", "unavailable")]
        );
        assert_eq!(report.to_json()["checks"][0]["status"], "ok");
        assert_eq!(report.to_json()["checks"][1]["status"], "unavailable");
    }

    #[tokio::test]
    async fn a_hanging_check_times_out_instead_of_hanging_the_probe() {
        let readiness = Readiness::new(Duration::from_millis(50)).with_check(Arc::new(Hangs));
        let report = readiness.run().await;
        assert_eq!(
            report.checks,
            vec![CheckOutcome {
                name: "slow",
                ok: false
            }]
        );
    }

    #[tokio::test]
    async fn tcp_probe_follows_the_listener() {
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let port = listener.local_addr().expect("addr").port();
        let probe = TcpProbe::new("database", "127.0.0.1", port);
        assert!(probe.check().await.is_ok());
        drop(listener);
        let error = probe.check().await.expect_err("nothing listens any more");
        assert!(error.to_string().starts_with("cannot connect"), "{error}");
    }

    #[test]
    fn readiness_is_derived_from_the_config() {
        let load = |url: Option<&str>| {
            let url = url.map(str::to_owned);
            Config::load(
                move |name| (name == "DATABASE_URL").then(|| url.clone()).flatten(),
                |_| Err(std::io::ErrorKind::NotFound.into()),
            )
            .expect("valid")
        };
        let without = format!("{:?}", Readiness::from_config(&load(None)));
        assert!(!without.contains("database"), "{without}");
        let with = format!(
            "{:?}",
            Readiness::from_config(&load(Some("postgres://u:p@h:1/d")))
        );
        assert!(with.contains("database") && !with.contains("u:p"), "{with}");
    }
}
