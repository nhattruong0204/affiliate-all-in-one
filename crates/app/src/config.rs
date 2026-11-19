//! Typed configuration, read once at startup and validated as a whole.
//!
//! Every problem is collected and reported together so a bad deployment fails once with the full
//! list instead of one variable per restart. Secrets are never echoed: not in errors, not in
//! `Debug` output.
//!
//! | Variable | Default | Notes |
//! |---|---|---|
//! | `APP_ENV` | `development` | `production` tightens validation (see below) |
//! | `RUST_LOG` | `info` | `tracing` filter directive |
//! | `LOG_FORMAT` | `json` in production, `text` otherwise | `json` or `text` |
//! | `WEB_BIND` | `127.0.0.1:8080` | public listener |
//! | `METRICS_BIND` | `127.0.0.1:9100` | internal listener; must differ from `WEB_BIND` |
//! | `READINESS_TIMEOUT_MS` | `2000` | per-check budget, 100–10000 |
//! | `DATABASE_URL` / `DATABASE_URL_FILE` | unset | `postgres://…`; the file form reads `/run/secrets/*` |
//!
//! In production the database is required and secrets must come from a `*_FILE` path, never from
//! an inline environment variable (environment values leak through `/proc`, crash reports and
//! `docker inspect`).

use std::fmt;
use std::io;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::time::Duration;

const DEFAULT_WEB_BIND: SocketAddr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 8080);
const DEFAULT_METRICS_BIND: SocketAddr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 9100);
const DEFAULT_READINESS_TIMEOUT_MS: u64 = 2_000;
const READINESS_TIMEOUT_RANGE_MS: std::ops::RangeInclusive<u64> = 100..=10_000;
const DEFAULT_PG_PORT: u16 = 5432;

/// Deployment profile.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AppEnv {
    Development,
    Production,
}

/// Log line encoding.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LogFormat {
    /// One JSON object per line, with PII redaction. The production format.
    Json,
    /// Human-readable lines for local development.
    Text,
}

/// A value that must never appear in logs or error messages.
#[derive(Clone, PartialEq, Eq)]
pub struct Secret(String);

impl Secret {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// The raw value. Call only where it is handed to the consumer (e.g. the database pool).
    pub fn expose(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for Secret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Secret(***)")
    }
}

/// Database settings. The URL is a secret because it embeds the password.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DatabaseConfig {
    pub url: Secret,
    /// Host and port parsed from the URL, for reachability probes.
    pub host: String,
    pub port: u16,
}

/// Everything the binaries need before they start serving.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Config {
    pub env: AppEnv,
    pub log_filter: String,
    pub log_format: LogFormat,
    pub web_bind: SocketAddr,
    pub metrics_bind: SocketAddr,
    pub readiness_timeout: Duration,
    pub database: Option<DatabaseConfig>,
}

/// All validation problems found, none of which contains a secret value.
#[derive(Debug, thiserror::Error)]
#[error("invalid configuration: {}", .0.join("; "))]
pub struct ConfigError(pub Vec<String>);

impl Config {
    /// Reads the process environment and secret files.
    ///
    /// # Errors
    /// Returns every validation problem found, see [`Config::load`].
    pub fn from_env() -> Result<Self, ConfigError> {
        Self::load(|name| std::env::var(name).ok(), read_secret_file)
    }

    /// Builds the configuration from `var` (environment) and `read_file` (secret files); both are
    /// injected so tests need neither real variables nor real files.
    ///
    /// # Errors
    /// Returns a [`ConfigError`] listing every invalid or missing value.
    pub fn load(
        var: impl Fn(&str) -> Option<String>,
        read_file: impl Fn(&str) -> io::Result<String>,
    ) -> Result<Self, ConfigError> {
        let mut issues = Vec::new();
        let get = |name: &str| {
            var(name)
                .map(|v| v.trim().to_owned())
                .filter(|v| !v.is_empty())
        };

        let env = match get("APP_ENV").as_deref() {
            None | Some("development" | "dev") => AppEnv::Development,
            Some("production" | "prod") => AppEnv::Production,
            Some(other) => {
                issues.push(format!(
                    "APP_ENV must be 'development' or 'production', got '{other}'"
                ));
                AppEnv::Development
            }
        };

        let log_format = match (get("LOG_FORMAT").as_deref(), env) {
            (None, AppEnv::Production) | (Some("json"), _) => LogFormat::Json,
            (None, AppEnv::Development) | (Some("text"), _) => LogFormat::Text,
            (Some(other), _) => {
                issues.push(format!(
                    "LOG_FORMAT must be 'json' or 'text', got '{other}'"
                ));
                LogFormat::Json
            }
        };

        let web_bind = parse_addr(&get, "WEB_BIND", DEFAULT_WEB_BIND, &mut issues);
        let metrics_bind = parse_addr(&get, "METRICS_BIND", DEFAULT_METRICS_BIND, &mut issues);
        if web_bind == metrics_bind {
            issues.push(
                "METRICS_BIND must differ from WEB_BIND: metrics are internal only".to_owned(),
            );
        }

        let readiness_timeout_ms = match get("READINESS_TIMEOUT_MS") {
            None => DEFAULT_READINESS_TIMEOUT_MS,
            Some(raw) => match raw.parse::<u64>() {
                Ok(ms) if READINESS_TIMEOUT_RANGE_MS.contains(&ms) => ms,
                _ => {
                    issues.push(format!(
                        "READINESS_TIMEOUT_MS must be an integer between {} and {}",
                        READINESS_TIMEOUT_RANGE_MS.start(),
                        READINESS_TIMEOUT_RANGE_MS.end()
                    ));
                    DEFAULT_READINESS_TIMEOUT_MS
                }
            },
        };

        let database = match secret("DATABASE_URL", env, &get, &read_file) {
            Ok(Some(url)) => {
                if let Some((host, port)) = parse_postgres_endpoint(url.expose()) {
                    Some(DatabaseConfig { url, host, port })
                } else {
                    issues.push(
                        "DATABASE_URL must look like postgres://user:password@host:port/database"
                            .to_owned(),
                    );
                    None
                }
            }
            Ok(None) => {
                if env == AppEnv::Production {
                    issues.push("DATABASE_URL_FILE is required in production".to_owned());
                }
                None
            }
            Err(issue) => {
                issues.push(issue);
                None
            }
        };

        if !issues.is_empty() {
            return Err(ConfigError(issues));
        }
        Ok(Self {
            env,
            log_filter: get("RUST_LOG").unwrap_or_else(|| "info".to_owned()),
            log_format,
            web_bind,
            metrics_bind,
            readiness_timeout: Duration::from_millis(readiness_timeout_ms),
            database,
        })
    }
}

fn read_secret_file(path: &str) -> io::Result<String> {
    std::fs::read_to_string(path)
}

fn parse_addr(
    get: &impl Fn(&str) -> Option<String>,
    name: &str,
    default: SocketAddr,
    issues: &mut Vec<String>,
) -> SocketAddr {
    match get(name) {
        None => default,
        Some(raw) => raw.parse().unwrap_or_else(|_| {
            issues.push(format!(
                "{name} must be a socket address such as {default}, got '{raw}'"
            ));
            default
        }),
    }
}

/// Reads `name` (inline) or `name_FILE` (path to a secret file). Returns a message that never
/// contains the secret itself.
fn secret(
    name: &str,
    env: AppEnv,
    get: &impl Fn(&str) -> Option<String>,
    read_file: &impl Fn(&str) -> io::Result<String>,
) -> Result<Option<Secret>, String> {
    let file_name = format!("{name}_FILE");
    match (get(name), get(&file_name)) {
        (Some(_), Some(_)) => Err(format!("set either {name} or {file_name}, not both")),
        (None, Some(path)) => match read_file(&path) {
            Ok(content) if content.trim().is_empty() => {
                Err(format!("{file_name} ({path}) points to an empty file"))
            }
            Ok(content) => Ok(Some(Secret::new(content.trim()))),
            Err(error) => Err(format!(
                "cannot read {file_name} ({path}): {}",
                error.kind()
            )),
        },
        (Some(value), None) => match env {
            AppEnv::Production => Err(format!(
                "in production {name} must come from a secret file: set {file_name}"
            )),
            AppEnv::Development => Ok(Some(Secret::new(value))),
        },
        (None, None) => Ok(None),
    }
}

/// Host and port of a `postgres://` URL (port defaults to 5432). Credentials are skipped, never
/// returned.
fn parse_postgres_endpoint(url: &str) -> Option<(String, u16)> {
    let rest = url
        .strip_prefix("postgres://")
        .or_else(|| url.strip_prefix("postgresql://"))?;
    let authority = rest.split(['/', '?', '#']).next()?;
    let host_port = authority.rsplit_once('@').map_or(authority, |(_, hp)| hp);
    let (host, port) = if let Some(bracketed) = host_port.strip_prefix('[') {
        let (host, after) = bracketed.split_once(']')?;
        let port = if after.is_empty() {
            None
        } else {
            Some(after.strip_prefix(':')?)
        };
        (host, port)
    } else {
        match host_port.rsplit_once(':') {
            Some((host, port)) => (host, Some(port)),
            None => (host_port, None),
        }
    };
    if host.is_empty() {
        return None;
    }
    let port = match port {
        None => DEFAULT_PG_PORT,
        Some(raw) => raw.parse().ok()?,
    };
    Some((host.to_owned(), port))
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    fn load(vars: &[(&str, &str)], files: &[(&str, &str)]) -> Result<Config, ConfigError> {
        let vars: HashMap<String, String> = vars
            .iter()
            .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
            .collect();
        let files: HashMap<String, String> = files
            .iter()
            .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
            .collect();
        Config::load(
            |name| vars.get(name).cloned(),
            |path| {
                files
                    .get(path)
                    .cloned()
                    .ok_or_else(|| io::Error::from(io::ErrorKind::NotFound))
            },
        )
    }

    fn issues(vars: &[(&str, &str)], files: &[(&str, &str)]) -> Vec<String> {
        load(vars, files).expect_err("must be invalid").0
    }

    #[test]
    fn empty_environment_is_a_valid_development_setup() {
        let config = load(&[], &[]).expect("valid");
        assert_eq!(config.env, AppEnv::Development);
        assert_eq!(config.log_format, LogFormat::Text);
        assert_eq!(config.log_filter, "info");
        assert_eq!(config.web_bind, "127.0.0.1:8080".parse().unwrap());
        assert_eq!(config.metrics_bind, "127.0.0.1:9100".parse().unwrap());
        assert_eq!(config.readiness_timeout, Duration::from_secs(2));
        assert_eq!(config.database, None);
    }

    #[test]
    fn production_reads_the_database_url_from_a_secret_file() {
        let config = load(
            &[
                ("APP_ENV", "production"),
                ("DATABASE_URL_FILE", "/run/secrets/database_url"),
                ("WEB_BIND", "0.0.0.0:8080"),
                ("METRICS_BIND", "0.0.0.0:9100"),
            ],
            &[(
                "/run/secrets/database_url",
                "postgres://aao:s3cret@db.internal:6543/aao\n",
            )],
        )
        .expect("valid");
        assert_eq!(config.env, AppEnv::Production);
        assert_eq!(
            config.log_format,
            LogFormat::Json,
            "json is the production default"
        );
        let db = config.database.expect("database configured");
        assert_eq!(
            db.url.expose(),
            "postgres://aao:s3cret@db.internal:6543/aao"
        );
        assert_eq!((db.host.as_str(), db.port), ("db.internal", 6543));
    }

    #[test]
    fn production_rejects_inline_secrets_and_a_missing_database() {
        let problems = issues(
            &[
                ("APP_ENV", "production"),
                ("DATABASE_URL", "postgres://aao:topsecret@db/aao"),
            ],
            &[],
        );
        assert_eq!(problems.len(), 1, "{problems:?}");
        assert!(problems[0].contains("DATABASE_URL_FILE"), "{problems:?}");
        assert!(!problems[0].contains("topsecret"));

        let problems = issues(&[("APP_ENV", "production")], &[]);
        assert!(
            problems[0].contains("required in production"),
            "{problems:?}"
        );
    }

    #[test]
    fn development_accepts_an_inline_url_but_not_both_forms() {
        let config = load(&[("DATABASE_URL", "postgres://u:p@localhost/aao")], &[]).expect("valid");
        assert_eq!(config.database.expect("db").port, 5432, "default port");

        let problems = issues(
            &[
                ("DATABASE_URL", "postgres://u:p@localhost/aao"),
                ("DATABASE_URL_FILE", "/run/secrets/x"),
            ],
            &[],
        );
        assert!(problems[0].contains("not both"), "{problems:?}");
    }

    #[test]
    fn secret_file_problems_never_echo_the_value() {
        let problems = issues(&[("DATABASE_URL_FILE", "/run/secrets/missing")], &[]);
        assert!(problems[0].contains("cannot read"), "{problems:?}");
        let problems = issues(
            &[("DATABASE_URL_FILE", "/run/secrets/db")],
            &[("/run/secrets/db", "  \n")],
        );
        assert!(problems[0].contains("empty file"), "{problems:?}");
        let problems = issues(
            &[("DATABASE_URL_FILE", "/run/secrets/db")],
            &[("/run/secrets/db", "mysql://u:hunter2@h/db")],
        );
        assert!(
            problems[0].contains("DATABASE_URL must look like"),
            "{problems:?}"
        );
        assert!(!problems.join(" ").contains("hunter2"));
    }

    #[test]
    fn every_problem_is_reported_at_once() {
        let problems = issues(
            &[
                ("APP_ENV", "staging"),
                ("LOG_FORMAT", "xml"),
                ("WEB_BIND", "localhost"),
                ("READINESS_TIMEOUT_MS", "5"),
            ],
            &[],
        );
        assert_eq!(problems.len(), 4, "{problems:?}");
        let error = ConfigError(problems).to_string();
        assert!(
            error.starts_with("invalid configuration: APP_ENV"),
            "{error}"
        );
    }

    #[test]
    fn metrics_listener_cannot_share_the_public_address() {
        let problems = issues(
            &[
                ("WEB_BIND", "127.0.0.1:9100"),
                ("METRICS_BIND", "127.0.0.1:9100"),
            ],
            &[],
        );
        assert!(problems[0].contains("must differ"), "{problems:?}");
    }

    #[test]
    fn postgres_endpoints_are_parsed_without_leaking_credentials() {
        let cases = [
            ("postgres://u:p@h/db", Some(("h", 5432))),
            (
                "postgresql://u:p@h:6000/db?sslmode=require",
                Some(("h", 6000)),
            ),
            ("postgres://h", Some(("h", 5432))),
            ("postgres://u:p%40ss@[::1]:5433/db", Some(("::1", 5433))),
            ("postgres://u:p@@h:1/db", Some(("h", 1))),
            ("postgres://u:p@h:notaport/db", None),
            ("postgres://u:p@:5432/db", None),
            ("postgres:///db", None),
            ("mysql://u:p@h/db", None),
            ("", None),
        ];
        for (url, expected) in cases {
            let got = parse_postgres_endpoint(url);
            assert_eq!(
                got.as_ref().map(|(h, p)| (h.as_str(), *p)),
                expected,
                "{url}"
            );
        }
    }

    #[test]
    fn secrets_are_masked_in_debug_output() {
        let config = load(&[("DATABASE_URL", "postgres://u:hunter2@h/db")], &[]).expect("valid");
        let debug = format!("{config:?}");
        assert!(!debug.contains("hunter2"), "{debug}");
        assert!(debug.contains("Secret(***)"), "{debug}");
    }
}
