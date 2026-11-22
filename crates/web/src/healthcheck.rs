//! `web healthcheck`: the container health probe (AAO-092).
//!
//! The runtime image is distroless (no shell, no `curl`), so a compose `healthcheck` cannot shell
//! out. Instead the binary probes itself: a plain-HTTP `GET /healthz` on its own public listener,
//! exit status `0` when it answers `200`, non-zero otherwise. `/healthz` is liveness only, so a
//! database outage never makes the container "unhealthy" (that is `/readyz`'s job, watched by the
//! external uptime monitor).
//!
//! Deliberately `std` only: no runtime, no configuration loading and no logging, so it starts
//! fast and cannot fail for configuration reasons.

use std::io::{Read, Write};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, TcpStream};
use std::time::Duration;

const TIMEOUT: Duration = Duration::from_secs(2);
const DEFAULT_BIND: &str = "127.0.0.1:8080";
const REQUEST: &[u8] = b"GET /healthz HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n";
const OK_STATUS: &[u8; 12] = b"HTTP/1.1 200";

/// The address to probe for a `WEB_BIND` value. An unspecified bind address (`0.0.0.0`, `::`)
/// means "every interface", which is reachable through loopback.
///
/// # Errors
/// Returns a message when the value is not `host:port` with a literal IP.
pub fn probe_addr(web_bind: Option<&str>) -> Result<SocketAddr, String> {
    let raw = web_bind
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(DEFAULT_BIND);
    let mut addr: SocketAddr = raw
        .parse()
        .map_err(|_| format!("WEB_BIND '{raw}' is not ip:port"))?;
    if addr.ip().is_unspecified() {
        addr.set_ip(match addr.ip() {
            IpAddr::V4(_) => IpAddr::V4(Ipv4Addr::LOCALHOST),
            IpAddr::V6(_) => IpAddr::V6(Ipv6Addr::LOCALHOST),
        });
    }
    Ok(addr)
}

/// Sends `GET /healthz` to `addr` and requires a `200`.
///
/// # Errors
/// Returns a message when the connection fails, times out or the status is not `200`.
pub fn probe(addr: SocketAddr) -> Result<(), String> {
    let mut stream = TcpStream::connect_timeout(&addr, TIMEOUT)
        .map_err(|error| format!("cannot connect to {addr}: {error}"))?;
    stream
        .set_read_timeout(Some(TIMEOUT))
        .and_then(|()| stream.set_write_timeout(Some(TIMEOUT)))
        .map_err(|error| format!("cannot set timeouts: {error}"))?;
    stream
        .write_all(REQUEST)
        .map_err(|error| format!("cannot send request: {error}"))?;
    let mut status = [0_u8; OK_STATUS.len()];
    stream
        .read_exact(&mut status)
        .map_err(|error| format!("no HTTP response: {error}"))?;
    if &status == OK_STATUS {
        Ok(())
    } else {
        Err(format!(
            "/healthz answered '{}', expected '200'",
            String::from_utf8_lossy(&status)
        ))
    }
}

/// Probes the listener named by `WEB_BIND` (default `127.0.0.1:8080`).
///
/// # Errors
/// See [`probe_addr`] and [`probe`].
pub fn run() -> Result<(), String> {
    probe(probe_addr(std::env::var("WEB_BIND").ok().as_deref())?)
}

#[cfg(test)]
mod tests {
    use std::net::TcpListener;
    use std::thread;

    use super::*;

    /// Serves one canned response on a loopback port and returns the address.
    fn serve_once(response: &'static [u8]) -> SocketAddr {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
        let addr = listener.local_addr().expect("addr");
        thread::spawn(move || {
            if let Ok((mut stream, _)) = listener.accept() {
                let mut request = [0_u8; 256];
                let _ = stream.read(&mut request);
                let _ = stream.write_all(response);
            }
        });
        addr
    }

    #[test]
    fn a_200_from_healthz_passes() {
        let addr = serve_once(b"HTTP/1.1 200 OK\r\ncontent-length: 2\r\n\r\nok");
        assert_eq!(probe(addr), Ok(()));
    }

    #[test]
    fn any_other_status_fails_and_says_which() {
        let addr = serve_once(b"HTTP/1.1 503 Service Unavailable\r\n\r\n");
        let error = probe(addr).expect_err("503 must fail");
        assert!(error.contains("HTTP/1.1 503"), "{error}");
    }

    #[test]
    fn a_closed_port_fails() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
        let addr = listener.local_addr().expect("addr");
        drop(listener);
        assert!(probe(addr).is_err());
    }

    #[test]
    fn a_server_that_hangs_up_without_answering_fails() {
        let addr = serve_once(b"");
        assert!(probe(addr).is_err());
    }

    #[test]
    fn the_bind_address_maps_to_loopback_and_defaults() {
        let parse = |v: Option<&str>| probe_addr(v).expect("valid");
        assert_eq!(parse(None), "127.0.0.1:8080".parse().unwrap());
        assert_eq!(parse(Some("  ")), "127.0.0.1:8080".parse().unwrap());
        assert_eq!(
            parse(Some("0.0.0.0:8080")),
            "127.0.0.1:8080".parse().unwrap()
        );
        assert_eq!(parse(Some("[::]:9000")), "[::1]:9000".parse().unwrap());
        assert_eq!(parse(Some("10.1.2.3:80")), "10.1.2.3:80".parse().unwrap());
        assert!(probe_addr(Some("localhost:8080")).is_err());
        assert!(probe_addr(Some("8080")).is_err());
    }
}
