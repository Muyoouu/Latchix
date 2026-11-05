use std::collections::HashSet;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};

use serde::Deserialize;

/// Result of one path check.
#[derive(Debug)]
pub struct PathStatus {
    /// Human-readable label.
    pub label: String,
    /// Whether the check passed.
    pub ok: bool,
    /// Detail for success or failure.
    pub detail: String,
}

/// Configuration load and validation error.
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    /// The configuration file could not be read.
    #[error("cannot read {path}: {source}")]
    Read {
        /// Configuration path.
        path: PathBuf,
        /// Underlying I/O error.
        source: std::io::Error,
    },

    /// The TOML was invalid.
    #[error("invalid TOML: {0}")]
    Parse(#[from] toml::de::Error),

    /// A value was invalid.
    #[error("invalid configuration: {0}")]
    Invalid(String),

    /// A directory could not be created.
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

/// Top-level configuration.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    /// SSH listener settings.
    pub server: ServerConfig,
    /// HTTP API settings.
    pub api: ApiConfig,
    /// Filesystem storage settings.
    pub storage: StorageConfig,
    /// SQLite settings.
    pub database: DatabaseConfig,
    /// Tenancy settings.
    pub tenancy: TenancyConfig,
    /// ACL settings.
    #[serde(default)]
    pub acl: AclConfig,
    /// Audit settings.
    #[serde(default)]
    pub audit: AuditConfig,
    /// Connection limits.
    #[serde(default)]
    pub limits: LimitsConfig,
    /// Admin UI settings.
    #[serde(default)]
    pub ui: UiConfig,
}

/// SSH listener settings.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ServerConfig {
    /// Address the SSH listener binds to.
    pub bind: String,
    /// Host key files.
    pub host_keys: Vec<PathBuf>,
}

/// HTTP API settings.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ApiConfig {
    /// Unix socket path.
    pub unix_socket: Option<PathBuf>,
    /// TCP listen address.
    pub tcp: Option<String>,
    /// Bootstrap token file.
    pub bootstrap_token_file: Option<PathBuf>,
}

/// Filesystem storage settings.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StorageConfig {
    /// Storage backend. Only `local` is supported in v0.1.
    pub backend: String,
    /// Root directory for tenant storage.
    pub root: PathBuf,
}

/// SQLite settings.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DatabaseConfig {
    /// SQLite URL.
    pub url: String,
    /// Busy timeout in milliseconds.
    pub busy_timeout_ms: u64,
    /// Synchronous mode.
    pub synchronous: String,
    /// Maximum read connections.
    pub max_read_connections: u32,
}

/// Tenancy settings.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TenancyConfig {
    /// Slug seeded as the default tenant.
    pub default_slug: String,
}

/// ACL settings.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AclConfig {
    /// Safety-net refresh interval for ACL snapshots.
    pub snapshot_refresh_seconds: u64,
}

impl Default for AclConfig {
    fn default() -> Self {
        Self {
            snapshot_refresh_seconds: 5,
        }
    }
}

/// Audit settings.
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuditConfig {
    /// Optional JSONL audit path.
    pub jsonl_path: Option<PathBuf>,
}

/// Connection limit settings.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LimitsConfig {
    /// Maximum sessions per user.
    pub max_sessions_per_user: u32,
    /// Maximum connections per IP address.
    pub max_connections_per_ip: u32,
}

impl Default for LimitsConfig {
    fn default() -> Self {
        Self {
            max_sessions_per_user: 10,
            max_connections_per_ip: 50,
        }
    }
}

/// Admin UI settings.
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UiConfig {
    /// Whether the admin UI is enabled.
    pub enabled: bool,
}

impl Config {
    /// Loads and validates configuration from a TOML file.
    pub fn load(path: &Path) -> Result<Self, ConfigError> {
        let text = std::fs::read_to_string(path).map_err(|source| ConfigError::Read {
            path: path.to_path_buf(),
            source,
        })?;
        Self::parse(&text)
    }

    /// Parses and validates configuration from TOML text.
    pub fn parse(text: &str) -> Result<Self, ConfigError> {
        let config: Self = toml::from_str(text)?;
        config.validate()?;
        Ok(config)
    }

    /// Validates cross-field constraints.
    pub fn validate(&self) -> Result<(), ConfigError> {
        let server_bind = self
            .server
            .bind
            .parse::<SocketAddr>()
            .map_err(|error| ConfigError::Invalid(format!("server.bind: {error}")))?;
        if server_bind.port() == 0 {
            return Err(ConfigError::Invalid(
                "server.bind must not use port zero".into(),
            ));
        }

        if self.server.host_keys.is_empty() {
            return Err(ConfigError::Invalid(
                "server.host_keys must contain at least one key".into(),
            ));
        }

        let mut seen_keys = HashSet::new();
        for path in &self.server.host_keys {
            if !path.is_absolute() {
                return Err(ConfigError::Invalid(format!(
                    "server.host_keys must be absolute: {}",
                    path.display()
                )));
            }
            if !seen_keys.insert(path) {
                return Err(ConfigError::Invalid(format!(
                    "server.host_keys contains a duplicate: {}",
                    path.display()
                )));
            }
        }

        if self.api.tcp.is_none() && self.api.unix_socket.is_none() {
            return Err(ConfigError::Invalid(
                "api.tcp or api.unix_socket must be set".into(),
            ));
        }

        if let Some(tcp) = &self.api.tcp {
            let address = tcp
                .parse::<SocketAddr>()
                .map_err(|error| ConfigError::Invalid(format!("api.tcp: {error}")))?;
            if address.port() == 0 {
                return Err(ConfigError::Invalid(
                    "api.tcp must not use port zero".into(),
                ));
            }
        }

        if let Some(path) = &self.api.unix_socket
            && !path.is_absolute()
        {
            return Err(ConfigError::Invalid(format!(
                "api.unix_socket must be absolute: {}",
                path.display()
            )));
        }

        if let Some(path) = &self.api.bootstrap_token_file
            && !path.is_absolute()
        {
            return Err(ConfigError::Invalid(format!(
                "api.bootstrap_token_file must be absolute: {}",
                path.display()
            )));
        }

        if self.storage.backend != "local" {
            return Err(ConfigError::Invalid(
                "storage.backend must be local in v0.1".into(),
            ));
        }

        if !self.storage.root.is_absolute() {
            return Err(ConfigError::Invalid(format!(
                "storage.root must be absolute: {}",
                self.storage.root.display()
            )));
        }
        if self.storage.root == Path::new("/") {
            return Err(ConfigError::Invalid(
                "storage.root must not be the filesystem root".into(),
            ));
        }

        self.database_path()?;
        if self.database.busy_timeout_ms == 0 {
            return Err(ConfigError::Invalid(
                "database.busy_timeout_ms must be greater than zero".into(),
            ));
        }
        if !matches!(self.database.synchronous.as_str(), "full" | "normal") {
            return Err(ConfigError::Invalid(
                "database.synchronous must be full or normal".into(),
            ));
        }
        if self.database.max_read_connections == 0 {
            return Err(ConfigError::Invalid(
                "database.max_read_connections must be greater than zero".into(),
            ));
        }

        if !valid_slug(&self.tenancy.default_slug) {
            return Err(ConfigError::Invalid(
                "tenancy.default_slug must be a lowercase DNS label".into(),
            ));
        }

        if self.acl.snapshot_refresh_seconds == 0 {
            return Err(ConfigError::Invalid(
                "acl.snapshot_refresh_seconds must be greater than zero".into(),
            ));
        }

        if let Some(path) = &self.audit.jsonl_path
            && !path.is_absolute()
        {
            return Err(ConfigError::Invalid(format!(
                "audit.jsonl_path must be absolute: {}",
                path.display()
            )));
        }

        if self.limits.max_sessions_per_user == 0 {
            return Err(ConfigError::Invalid(
                "limits.max_sessions_per_user must be greater than zero".into(),
            ));
        }
        if self.limits.max_connections_per_ip == 0 {
            return Err(ConfigError::Invalid(
                "limits.max_connections_per_ip must be greater than zero".into(),
            ));
        }

        Ok(())
    }

    /// Returns the SQLite file path.
    pub fn database_path(&self) -> Result<PathBuf, ConfigError> {
        let raw = self
            .database
            .url
            .strip_prefix("sqlite://")
            .or_else(|| self.database.url.strip_prefix("sqlite:"))
            .ok_or_else(|| ConfigError::Invalid("database.url must start with sqlite://".into()))?;

        if raw.is_empty() || raw == ":memory:" {
            return Err(ConfigError::Invalid(
                "database.url must point to a file".into(),
            ));
        }

        let path = PathBuf::from(raw);
        if !path.is_absolute() {
            return Err(ConfigError::Invalid(format!(
                "database.url must be absolute: {}",
                path.display()
            )));
        }

        Ok(path)
    }

    /// Creates the storage root, database directory, and API socket directory.
    ///
    /// Existing directories are left with their current permissions.
    pub fn ensure_directories(&self) -> Result<(), ConfigError> {
        create_private_dir(&self.storage.root)?;

        let database_path = self.database_path()?;
        if let Some(parent) = database_path.parent() {
            create_private_dir(parent)?;
        }

        if let Some(path) = &self.api.unix_socket
            && let Some(parent) = path.parent()
        {
            create_private_dir(parent)?;
        }

        Ok(())
    }

    /// Checks the runtime paths that must exist before the server starts.
    #[must_use]
    pub fn path_statuses(&self) -> Vec<PathStatus> {
        let mut statuses = vec![directory_status("storage root", &self.storage.root)];

        match self.database_path() {
            Ok(path) => {
                if let Some(parent) = path.parent() {
                    statuses.push(directory_status("database directory", parent));
                }
                if path.exists() {
                    statuses.push(file_status("database file", &path));
                }
            }
            Err(error) => statuses.push(PathStatus {
                label: "database path".into(),
                ok: false,
                detail: error.to_string(),
            }),
        }

        for (index, path) in self.server.host_keys.iter().enumerate() {
            let mut status = file_status(format!("host key {}", index + 1), path);
            if status.ok
                && let Err(error) = latchix_sftp::validate_host_key(path)
            {
                status.ok = false;
                status.detail = error.to_string();
            }
            statuses.push(status);
        }

        if let Some(path) = &self.api.unix_socket
            && let Some(parent) = path.parent()
        {
            statuses.push(directory_status("api socket directory", parent));
        }

        if let Some(path) = &self.audit.jsonl_path
            && let Some(parent) = path.parent()
        {
            statuses.push(directory_status("audit directory", parent));
        }

        statuses
    }
}

fn valid_slug(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 63
        && !value.starts_with('-')
        && !value.ends_with('-')
        && value.chars().all(|character| {
            character.is_ascii_lowercase() || character.is_ascii_digit() || character == '-'
        })
}

fn create_private_dir(path: &Path) -> Result<(), std::io::Error> {
    let existed = path.exists();
    std::fs::create_dir_all(path)?;

    #[cfg(unix)]
    if !existed {
        use std::os::unix::fs::PermissionsExt;

        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700))?;
    }

    Ok(())
}

fn directory_status(label: impl Into<String>, path: &Path) -> PathStatus {
    let label = label.into();
    let metadata = match std::fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) => {
            return PathStatus {
                label,
                ok: false,
                detail: error.to_string(),
            };
        }
    };

    if metadata.file_type().is_symlink() {
        return PathStatus {
            label,
            ok: false,
            detail: format!("{} is a symlink", path.display()),
        };
    }

    if !metadata.is_dir() {
        return PathStatus {
            label,
            ok: false,
            detail: format!("{} is not a directory", path.display()),
        };
    }

    PathStatus {
        label,
        ok: true,
        detail: path.display().to_string(),
    }
}

fn file_status(label: impl Into<String>, path: &Path) -> PathStatus {
    let label = label.into();
    let metadata = match std::fs::metadata(path) {
        Ok(metadata) => metadata,
        Err(error) => {
            return PathStatus {
                label,
                ok: false,
                detail: error.to_string(),
            };
        }
    };

    if !metadata.is_file() {
        return PathStatus {
            label,
            ok: false,
            detail: format!("{} is not a file", path.display()),
        };
    }

    if let Err(error) = std::fs::File::open(path) {
        return PathStatus {
            label,
            ok: false,
            detail: error.to_string(),
        };
    }

    PathStatus {
        label,
        ok: true,
        detail: path.display().to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::{Config, ConfigError};

    const VALID: &str = r#"
[server]
bind = "127.0.0.1:2222"
host_keys = ["/etc/latchix/host_ed25519_key"]

[api]
unix_socket = "/run/latchix/api.sock"
tcp = "127.0.0.1:8080"
bootstrap_token_file = "/run/secrets/latchix_bootstrap_token"

[storage]
backend = "local"
root = "/var/lib/latchix/files"

[database]
url = "sqlite:///var/lib/latchix/latchix.db"
busy_timeout_ms = 5000
synchronous = "full"
max_read_connections = 8

[tenancy]
default_slug = "default"

[acl]
snapshot_refresh_seconds = 5

[audit]
jsonl_path = "/var/log/latchix/audit.jsonl"

[limits]
max_sessions_per_user = 10
max_connections_per_ip = 50

[ui]
enabled = false
"#;

    #[test]
    fn valid_config_parses() {
        let config = Config::parse(VALID).expect("valid config");
        assert_eq!(config.tenancy.default_slug, "default");
        assert_eq!(
            config.database_path().expect("database path"),
            std::path::PathBuf::from("/var/lib/latchix/latchix.db")
        );
    }

    #[test]
    fn relative_storage_root_is_rejected() {
        let text = VALID.replace("/var/lib/latchix/files", "relative/files");
        assert!(matches!(Config::parse(&text), Err(ConfigError::Invalid(_))));
    }

    #[test]
    fn in_memory_database_is_rejected() {
        let text = VALID.replace("sqlite:///var/lib/latchix/latchix.db", "sqlite::memory:");
        assert!(matches!(Config::parse(&text), Err(ConfigError::Invalid(_))));
    }

    #[test]
    fn uppercase_slug_is_rejected() {
        let text = VALID.replace("default_slug = \"default\"", "default_slug = \"Default\"");
        assert!(matches!(Config::parse(&text), Err(ConfigError::Invalid(_))));
    }

    #[test]
    fn duplicate_host_keys_are_rejected() {
        let text = VALID.replace(
            "host_keys = [\"/etc/latchix/host_ed25519_key\"]",
            "host_keys = [\"/etc/latchix/host_ed25519_key\", \"/etc/latchix/host_ed25519_key\"]",
        );
        assert!(matches!(Config::parse(&text), Err(ConfigError::Invalid(_))));
    }

    #[test]
    fn port_zero_is_rejected() {
        let text = VALID.replace("127.0.0.1:2222", "127.0.0.1:0");
        assert!(matches!(Config::parse(&text), Err(ConfigError::Invalid(_))));
        let text = VALID.replace("127.0.0.1:8080", "127.0.0.1:0");
        assert!(matches!(Config::parse(&text), Err(ConfigError::Invalid(_))));
    }

    #[test]
    fn filesystem_root_storage_is_rejected() {
        let text = VALID.replace("/var/lib/latchix/files", "/");
        assert!(matches!(Config::parse(&text), Err(ConfigError::Invalid(_))));
    }

    #[test]
    fn unknown_fields_are_rejected() {
        let text = format!("{VALID}\n[unknown]\nvalue = true\n");
        assert!(matches!(Config::parse(&text), Err(ConfigError::Parse(_))));
    }
}
