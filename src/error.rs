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
