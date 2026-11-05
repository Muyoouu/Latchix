#![forbid(unsafe_code)]

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub mod fs;
pub mod path;

pub use fs::{
    DirEntry, DirHandle, FileHandle, FileKind, FsError, Metadata, OpenOptions, SetAttrs, VirtualFs,
};
pub use path::{MAX_PATH_BYTES, MAX_SEGMENT_BYTES, PathError, VirtualPath};

macro_rules! id_type {
    ($name:ident, $doc:literal) => {
        #[doc = $doc]
        #[derive(
            Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize,
        )]
        #[serde(transparent)]
        pub struct $name(Uuid);

        impl $name {
            #[doc = concat!("Creates a new ", stringify!($name), " using a time-ordered UUID.")]
            #[must_use]
            pub fn new() -> Self {
                Self(Uuid::now_v7())
            }

            #[doc = concat!("Wraps an existing UUID as a ", stringify!($name), ".")]
            #[must_use]
            pub const fn from_uuid(value: Uuid) -> Self {
                Self(value)
            }

            #[doc = concat!("Returns the wrapped UUID.")]
            #[must_use]
            pub const fn as_uuid(&self) -> Uuid {
                self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                self.0.fmt(formatter)
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }

        impl FromStr for $name {
            type Err = uuid::Error;

            fn from_str(value: &str) -> Result<Self, Self::Err> {
                Uuid::parse_str(value).map(Self)
            }
        }
    };
}

id_type!(TenantId, "Opaque identifier for a tenant.");
id_type!(RootId, "Opaque identifier for a storage root.");

/// The tenant and root a request is allowed to act on.
///
/// A request context is always explicit. It is never read from a process global.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct TenantContext {
    tenant_id: TenantId,
    root_id: RootId,
}

impl TenantContext {
    /// Creates a tenant and root context.
    #[must_use]
    pub const fn new(tenant_id: TenantId, root_id: RootId) -> Self {
        Self { tenant_id, root_id }
    }

    /// Returns the tenant for this context.
    #[must_use]
    pub const fn tenant_id(&self) -> TenantId {
        self.tenant_id
    }

    /// Returns the root for this context.
    #[must_use]
    pub const fn root_id(&self) -> RootId {
        self.root_id
    }
}

#[cfg(test)]
mod tests {
    use super::{RootId, TenantContext, TenantId};

    #[test]
    fn tenant_id_roundtrips_through_text() {
        let id = TenantId::new();
        let parsed: TenantId = id.to_string().parse().expect("valid tenant id");
        assert_eq!(id, parsed);
    }

    #[test]
    fn root_id_roundtrips_through_text() {
        let id = RootId::new();
        let parsed: RootId = id.to_string().parse().expect("valid root id");
        assert_eq!(id, parsed);
    }

    #[test]
    fn tenant_context_keeps_both_ids() {
        let tenant_id = TenantId::new();
        let root_id = RootId::new();
        let context = TenantContext::new(tenant_id, root_id);
        assert_eq!(context.tenant_id(), tenant_id);
        assert_eq!(context.root_id(), root_id);
    }
}
