#![forbid(unsafe_code)]

use rusqlite::Connection;

/// Errors returned by the metadata store.
#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    /// SQLite returned an error.
    #[error("sqlite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
}

/// Returns the SQLite library version compiled into this build.
///
/// This proves the bundled SQLite library is available without a system
/// dependency.
pub fn sqlite_version() -> Result<String, StoreError> {
    let connection = Connection::open_in_memory()?;
    let version = connection.query_row("select sqlite_version()", [], |row| row.get(0))?;
    Ok(version)
}

#[cfg(test)]
mod tests {
    #[test]
    fn bundled_sqlite_is_available() {
        let version = super::sqlite_version().expect("bundled sqlite opens");
        assert!(
            version.starts_with('3'),
            "unexpected sqlite version: {version}"
        );
    }
}
