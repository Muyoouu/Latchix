#![forbid(unsafe_code)]

/// Version of the audit event payload schema.
pub const SCHEMA_VERSION: u32 = 1;

/// Returns the audit schema version emitted by this build.
#[must_use]
pub const fn schema_version() -> u32 {
    SCHEMA_VERSION
}

#[cfg(test)]
mod tests {
    #[test]
    fn schema_version_is_positive() {
        assert!(super::schema_version() >= 1);
    }
}
