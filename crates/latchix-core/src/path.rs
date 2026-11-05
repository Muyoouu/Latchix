use std::fmt;
use std::str::FromStr;

use unicode_normalization::UnicodeNormalization;

/// Maximum canonical virtual path length in bytes.
pub const MAX_PATH_BYTES: usize = 4096;

/// Maximum virtual path segment length in bytes.
pub const MAX_SEGMENT_BYTES: usize = 255;

/// A canonical, root-relative path inside a tenant root.
///
/// The path always starts with `/`. It has no trailing slash except for the
/// root `/`. It is NFC-normalized. It never contains `..`, NUL, or empty
/// segments. SFTP absolute paths such as `/etc/passwd` are virtual paths under
/// the tenant root, not host paths.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct VirtualPath(String);

/// Path parsing error.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum PathError {
    /// The input contains a NUL byte.
    #[error("path contains NUL")]
    Nul,

    /// The input contains `..`.
    #[error("path contains parent traversal")]
    ParentTraversal,

    /// The input contains an empty segment from repeated slashes.
    #[error("path contains an empty segment")]
    EmptySegment,

    /// The canonical path exceeds the maximum length.
    #[error("path is too long")]
    TooLong,

    /// One segment exceeds the maximum length.
    #[error("path segment is too long")]
    SegmentTooLong,
}

impl VirtualPath {
    /// Parses and canonicalizes a virtual path.
    pub fn parse(input: &str) -> Result<Self, PathError> {
        if input.contains('\0') {
            return Err(PathError::Nul);
        }

        let normalized: String = input.nfc().collect();
        if normalized.len() > MAX_PATH_BYTES {
            return Err(PathError::TooLong);
        }

        if normalized.is_empty() {
            return Ok(Self("/".into()));
        }

        let has_leading_slash = normalized.starts_with('/');
        let has_trailing_slash = normalized.ends_with('/');
        let parts: Vec<&str> = normalized.split('/').collect();
        let mut segments = Vec::new();

        for (index, segment) in parts.iter().enumerate() {
            if segment.is_empty() {
                let is_leading_empty = index == 0 && has_leading_slash;
                let is_trailing_empty = index + 1 == parts.len() && has_trailing_slash;
                if is_leading_empty || is_trailing_empty {
                    continue;
                }
                return Err(PathError::EmptySegment);
            }

            if *segment == "." {
                continue;
            }

            if *segment == ".." {
                return Err(PathError::ParentTraversal);
            }

            if segment.len() > MAX_SEGMENT_BYTES {
                return Err(PathError::SegmentTooLong);
            }

            segments.push(*segment);
        }

        let canonical = if segments.is_empty() {
            "/".to_string()
        } else {
            format!("/{}", segments.join("/"))
        };

        if canonical.len() > MAX_PATH_BYTES {
            return Err(PathError::TooLong);
        }

        Ok(Self(canonical))
    }

    /// Returns the root path `/`.
    #[must_use]
    pub fn root() -> Self {
        Self("/".into())
    }

    /// Returns the canonical path.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Returns whether this is the root path.
    #[must_use]
    pub fn is_root(&self) -> bool {
        self.0 == "/"
    }

    /// Iterates over path segments, excluding the root marker.
    pub fn segments(&self) -> impl Iterator<Item = &str> {
        self.0.split('/').filter(|segment| !segment.is_empty())
    }

    /// Returns the parent path, or `None` for root.
    #[must_use]
    pub fn parent(&self) -> Option<Self> {
        if self.is_root() {
            return None;
        }

        let without_leading = &self.0[1..];
        match without_leading.rfind('/') {
            Some(index) => Some(Self(format!("/{}", &without_leading[..index]))),
            None => Some(Self("/".into())),
        }
    }

    /// Returns the final segment, or `None` for root.
    #[must_use]
    pub fn file_name(&self) -> Option<&str> {
        if self.is_root() {
            return None;
        }

        self.0.rsplit('/').find(|segment| !segment.is_empty())
    }

    /// Returns the root-relative path used for `openat2`.
    ///
    /// The root path becomes `.` so every lookup stays relative to the root
    /// descriptor.
    #[must_use]
    pub fn as_relative(&self) -> &str {
        if self.is_root() { "." } else { &self.0[1..] }
    }
}

impl fmt::Display for VirtualPath {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

impl AsRef<str> for VirtualPath {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl FromStr for VirtualPath {
    type Err = PathError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::parse(value)
    }
}

#[cfg(test)]
mod tests {
    use super::{MAX_PATH_BYTES, MAX_SEGMENT_BYTES, PathError, VirtualPath};

    fn parse(value: &str) -> VirtualPath {
        VirtualPath::parse(value).expect("valid path")
    }

    #[test]
    fn root_forms() {
        assert_eq!(parse("").as_str(), "/");
        assert_eq!(parse("/").as_str(), "/");
        assert_eq!(parse(".").as_str(), "/");
        assert_eq!(parse("./").as_str(), "/");
        assert!(parse("/").is_root());
    }

    #[test]
    fn relative_and_absolute_forms_are_virtual() {
        assert_eq!(parse("foo").as_str(), "/foo");
        assert_eq!(parse("/foo").as_str(), "/foo");
        assert_eq!(parse("/etc/passwd").as_str(), "/etc/passwd");
        assert_eq!(parse("foo/bar").as_str(), "/foo/bar");
        assert_eq!(parse("./foo/./bar").as_str(), "/foo/bar");
        assert_eq!(parse("foo/").as_str(), "/foo");
    }

    #[test]
    fn parent_traversal_is_rejected() {
        assert_eq!(
            VirtualPath::parse("../etc/passwd"),
            Err(PathError::ParentTraversal)
        );
        assert_eq!(
            VirtualPath::parse("foo/../../bar"),
            Err(PathError::ParentTraversal)
        );
        assert_eq!(
            VirtualPath::parse("/foo/../bar"),
            Err(PathError::ParentTraversal)
        );
    }

    #[test]
    fn nul_is_rejected() {
        assert_eq!(VirtualPath::parse("foo\0bar"), Err(PathError::Nul));
    }

    #[test]
    fn repeated_slashes_are_rejected() {
        assert_eq!(VirtualPath::parse("foo//bar"), Err(PathError::EmptySegment));
        assert_eq!(VirtualPath::parse("//foo"), Err(PathError::EmptySegment));
        assert_eq!(
            VirtualPath::parse("/foo//bar"),
            Err(PathError::EmptySegment)
        );
        assert_eq!(VirtualPath::parse("///"), Err(PathError::EmptySegment));
        assert_eq!(VirtualPath::parse("foo//"), Err(PathError::EmptySegment));
    }

    #[test]
    fn segments_skip_root_marker() {
        let path = parse("/foo/bar");
        let segments: Vec<&str> = path.segments().collect();
        assert_eq!(segments, vec!["foo", "bar"]);
        assert_eq!(parse("/").segments().count(), 0);
    }

    #[test]
    fn parent_and_file_name() {
        let path = parse("/foo/bar");
        assert_eq!(path.parent().expect("parent").as_str(), "/foo");
        assert_eq!(path.file_name(), Some("bar"));
        assert_eq!(parse("/foo").parent().expect("parent").as_str(), "/");
        assert!(parse("/").parent().is_none());
        assert_eq!(parse("/").file_name(), None);
    }

    #[test]
    fn root_relative_form() {
        assert_eq!(parse("/").as_relative(), ".");
        assert_eq!(parse("/foo/bar").as_relative(), "foo/bar");
    }

    #[test]
    fn nfc_normalization() {
        let decomposed = "e\u{0301}";
        let composed = "\u{00e9}";
        assert_eq!(parse(decomposed).as_str(), "/\u{00e9}");
        assert_eq!(parse(decomposed), parse(composed));
    }

    #[test]
    fn display_and_as_ref_are_canonical() {
        let path = parse("/foo/bar/");
        assert_eq!(path.to_string(), "/foo/bar");
        assert_eq!(path.as_ref(), "/foo/bar");
        assert_eq!(path.as_str(), "/foo/bar");
    }

    #[test]
    fn long_path_and_segment_are_rejected() {
        let long_segment = "a".repeat(MAX_SEGMENT_BYTES + 1);
        assert_eq!(
            VirtualPath::parse(&long_segment),
            Err(PathError::SegmentTooLong)
        );

        let long_path = format!("/{}", "a/".repeat(MAX_PATH_BYTES));
        assert_eq!(VirtualPath::parse(&long_path), Err(PathError::TooLong));
    }
}
