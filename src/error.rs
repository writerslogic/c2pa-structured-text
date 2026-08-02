// Copyright 2026 WritersLogic. All rights reserved.
// Licensed under the Apache License, Version 2.0 or the MIT license,
// at your option.

use std::fmt;

#[derive(Debug)]
pub enum Error {
    /// No `-----BEGIN/END C2PA MANIFEST-----` block was located.
    NotFound,
    /// More than one manifest block was found. Per the specification the asset
    /// shall then be treated as if no manifests were located.
    MultipleBlocks,
    /// The block is present but the reference between the delimiters is empty.
    EmptyReference,
    /// The reference is neither a resolvable URI nor a `data:` URI.
    MalformedReference(String),
    /// The text uses bare CR (0x0D) line endings, which are unsupported by the
    /// structured-text binding method because they make line boundaries
    /// ambiguous. Convert to LF or CRLF before embedding or validating.
    BareCarriageReturn,
    /// A `data:application/c2pa;base64,` reference could not be Base64-decoded.
    ManifestDecode(crate::codec::DecodeError),
    /// The exclusion ranges of a data hash assertion are malformed: negative,
    /// out of order, overlapping, or extending past the end of the asset.
    /// Corresponds to `assertion.dataHash.malformed`.
    MalformedExclusion,
    /// The recomputed data hash did not match the value in the assertion.
    /// Corresponds to `assertion.dataHash.mismatch`.
    HashMismatch,
    /// A hash algorithm identifier outside the C2PA allowed list was requested.
    /// Corresponds to `algorithm.unsupported`.
    UnsupportedAlgorithm(String),
}

impl Error {
    /// The registered C2PA validation status code for this error, or `None`
    /// when the condition carries no status code.
    ///
    /// The specification defines no status codes specific to structured text —
    /// unlike HTML or unstructured text, which each have their own — so every
    /// locating outcome here returns `None`. Only the data-hash conditions map
    /// to codes, and those are the generic ones shared by every embedding
    /// method.
    ///
    /// Every crate in this family exposes this method, so a dispatcher handling
    /// several embedding methods can ask the same question of any of them.
    pub fn code(&self) -> Option<&'static str> {
        Some(match self {
            Self::MalformedExclusion => "assertion.dataHash.malformed",
            Self::HashMismatch => "assertion.dataHash.mismatch",
            Self::UnsupportedAlgorithm(_) => "algorithm.unsupported",
            Self::NotFound
            | Self::MultipleBlocks
            | Self::EmptyReference
            | Self::MalformedReference(_)
            | Self::BareCarriageReturn
            | Self::ManifestDecode(_) => return None,
        })
    }

    /// Whether this error means the asset carries no provenance at all, as
    /// opposed to provenance that was found and rejected.
    ///
    /// [`Error::MultipleBlocks`] counts: the specification requires an asset
    /// with more than one manifest block to be treated as if no manifests were
    /// located, rather than reported as a failure.
    pub fn is_no_manifest_located(&self) -> bool {
        matches!(self, Self::NotFound | Self::MultipleBlocks)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound => write!(f, "no manifest block found"),
            Self::MultipleBlocks => write!(f, "multiple manifest blocks found"),
            Self::EmptyReference => write!(f, "empty manifest reference"),
            Self::MalformedReference(s) => write!(f, "malformed manifest reference: {s}"),
            Self::BareCarriageReturn => {
                write!(
                    f,
                    "bare CR line endings are not supported; convert to LF or CRLF"
                )
            }
            Self::ManifestDecode(e) => write!(f, "manifest data URI is not valid base64: {e}"),
            Self::MalformedExclusion => write!(f, "data hash exclusion range is malformed"),
            Self::HashMismatch => write!(f, "data hash does not match the asset content"),
            Self::UnsupportedAlgorithm(a) => write!(f, "unsupported hash algorithm: {a}"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::ManifestDecode(e) => Some(e),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn all() -> Vec<Error> {
        vec![
            Error::NotFound,
            Error::MultipleBlocks,
            Error::EmptyReference,
            Error::MalformedReference("x".into()),
            Error::BareCarriageReturn,
            Error::MalformedExclusion,
            Error::HashMismatch,
            Error::UnsupportedAlgorithm("sha1".into()),
        ]
    }

    /// Guards against inventing a code. Structured text has no format-specific
    /// codes, so only the three generic ones may ever appear here.
    #[test]
    fn every_code_is_a_registered_identifier() {
        for e in all() {
            if let Some(code) = e.code() {
                assert!(
                    matches!(
                        code,
                        "assertion.dataHash.malformed"
                            | "assertion.dataHash.mismatch"
                            | "algorithm.unsupported"
                    ),
                    "{e:?} reports an unregistered code: {code}"
                );
            }
        }
    }

    #[test]
    fn no_structured_text_specific_code_is_emitted() {
        // `manifest.structuredText.*` was removed from the specification;
        // emitting one would be inventing specification.
        for e in all() {
            if let Some(code) = e.code() {
                assert!(!code.starts_with("manifest."), "{e:?} emits {code}");
            }
        }
    }

    #[test]
    fn locating_outcomes_carry_no_code() {
        for e in [Error::NotFound, Error::MultipleBlocks] {
            assert_eq!(e.code(), None, "{e:?} must not report a status code");
            assert!(
                e.is_no_manifest_located(),
                "{e:?} must classify as unsigned"
            );
        }
    }

    #[test]
    fn binding_failures_are_not_no_manifest_located() {
        for e in [
            Error::MalformedExclusion,
            Error::HashMismatch,
            Error::UnsupportedAlgorithm("sha1".into()),
        ] {
            assert!(!e.is_no_manifest_located(), "{e:?} misclassified");
            assert!(e.code().is_some(), "{e:?} should report a code");
        }
    }
}
