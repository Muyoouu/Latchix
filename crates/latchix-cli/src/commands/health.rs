use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::path::Path;
use std::time::Duration;

use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::net::{TcpStream, UnixStream};

use crate::config::Config;
use crate::error::AppError;

const PROBE_TIMEOUT: Duration = Duration::from_secs(2);

/// Checks configuration, runtime paths, and the running API.
pub async fn run(config_path: &Path) -> Result<(), AppError> {
    let config = Config::load(config_path)?;
    let statuses = config.path_statuses();
    let mut failures = 0_u32;

    for status in &statuses {
        if !status.ok {
            eprintln!("{}: {}", status.label, status.detail);
            failures += 1;
        }
    }

    if failures > 0 {
        return Err(AppError::Health(format!("{failures} path check(s) failed")));
    }

    if let Some(tcp) = &config.api.tcp {
        match probe_tcp(tcp).await {
            Ok(()) => {
                println!("latchix: healthy");
                return Ok(());
            }
            Err(error) if config.api.unix_socket.is_some() => {
                eprintln!("api tcp probe failed: {error}");
            }
            Err(error) => return Err(AppError::Health(format!("api tcp probe failed: {error}"))),
        }
    }

    if let Some(path) = &config.api.unix_socket {
        probe_unix(path)
            .await
            .map_err(|error| AppError::Health(format!("api unix probe failed: {error}")))?;
        println!("latchix: healthy");
        return Ok(());
    }

    Err(AppError::Health("no API listener configured".into()))
}

async fn probe_tcp(bind: &str) -> Result<(), String> {
    let address = bind
        .parse::<SocketAddr>()
        .map_err(|error| format!("invalid api.tcp: {error}"))?;
    let address = probe_address(address);
    let mut stream = tokio::time::timeout(PROBE_TIMEOUT, TcpStream::connect(address))
        .await
        .map_err(|_| format!("connect to {address} timed out"))?
        .map_err(|error| format!("connect to {address} failed: {error}"))?;

    probe_stream(&mut stream).await
}

async fn probe_unix(path: &Path) -> Result<(), String> {
    let mut stream = tokio::time::timeout(PROBE_TIMEOUT, UnixStream::connect(path))
        .await
        .map_err(|_| format!("connect to {} timed out", path.display()))?
        .map_err(|error| format!("connect to {} failed: {error}", path.display()))?;

    probe_stream(&mut stream).await
}

fn probe_address(address: SocketAddr) -> SocketAddr {
    match address.ip() {
        IpAddr::V4(ip) if ip.is_unspecified() => {
            SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), address.port())
        }
        IpAddr::V6(ip) if ip.is_unspecified() => {
            SocketAddr::new(IpAddr::V6(Ipv6Addr::LOCALHOST), address.port())
        }
        _ => address,
    }
}

async fn probe_stream<S>(stream: &mut S) -> Result<(), String>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    let request = "GET /readyz HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n";
    tokio::time::timeout(PROBE_TIMEOUT, stream.write_all(request.as_bytes()))
        .await
        .map_err(|_| "write timed out".to_string())?
        .map_err(|error| format!("write failed: {error}"))?;

    let mut response = Vec::new();
    tokio::time::timeout(PROBE_TIMEOUT, stream.read_to_end(&mut response))
        .await
        .map_err(|_| "read timed out".to_string())?
        .map_err(|error| format!("read failed: {error}"))?;

    let status = parse_status(&response)?;
    if status == 200 {
        Ok(())
    } else {
        Err(format!("/readyz returned {status}"))
    }
}

fn parse_status(response: &[u8]) -> Result<u16, String> {
    let line = response
        .split(|byte| *byte == b'\n')
        .next()
        .ok_or_else(|| "empty response".to_string())?;
    let line = std::str::from_utf8(line)
        .map_err(|error| format!("invalid status line: {error}"))?
        .trim_end();
    let mut parts = line.split_whitespace();
    let version = parts
        .next()
        .ok_or_else(|| "missing status version".to_string())?;
    if !version.starts_with("HTTP/") {
        return Err(format!("unexpected status line: {line}"));
    }
    parts
        .next()
        .ok_or_else(|| "missing status code".to_string())?
        .parse::<u16>()
        .map_err(|error| format!("invalid status code: {error}"))
}

#[cfg(test)]
mod tests {
    use super::parse_status;

    #[test]
    fn parses_http_status() {
        assert_eq!(
            parse_status(b"HTTP/1.1 200 OK\r\n\r\n").expect("status"),
            200
        );
        assert_eq!(
            parse_status(b"HTTP/1.1 503 Service Unavailable\r\n\r\n").expect("status"),
            503
        );
    }

    #[test]
    fn rejects_non_http_status() {
        assert!(parse_status(b"not http\r\n").is_err());
    }
}
