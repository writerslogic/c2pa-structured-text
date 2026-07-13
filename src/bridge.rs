// Copyright 2026 WritersLogic. All rights reserved.
// Licensed under the Apache License, Version 2.0 or the MIT license,
// at your option.

//! Validation bridge to `c2pa-rs`.
//!
//! This crate owns locating and extracting the manifest block and defining the
//! structured-text hard binding. It does **not** implement COSE signature
//! verification, certificate trust, or assertion validation; those are
//! delegated to `c2pa-rs`.
//!
//! The bridge extracts the reference, resolves it to a C2PA Manifest Store
//! (decoding a `data:` URI inline, or fetching a URL with the `remote`
//! feature), and hands the store plus the text stream to [`c2pa::Reader`],
//! which validates the signature, trust chain, and the `c2pa.hash.data` hard
//! binding against the raw bytes. Because the manifest block is excluded from
//! the hash, `c2pa-rs`'s raw-byte data-hash validation matches the binding this
//! crate computes.

use std::io::Cursor;

use c2pa::{Context, Reader};

use crate::error::Error;
use crate::extract::{classify_reference, extract_manifest, Reference};

/// The default media type used when handing a text stream to `c2pa-rs`.
pub const DEFAULT_FORMAT: &str = "text/plain";

/// An error from the validation bridge.
#[derive(Debug)]
pub enum BridgeError {
    /// Locating, extracting, or classifying the reference failed.
    Extract(Error),
    /// The reference is a URL but network resolution is not available. Enable
    /// the `remote` feature, or resolve it yourself and call
    /// [`validate_with_manifest`].
    RemoteNotEnabled(String),
    /// Resolving a remote manifest over the network failed.
    #[cfg(feature = "remote")]
    Fetch(String),
    /// `c2pa-rs` rejected the manifest data or errored during validation.
    C2pa(c2pa::Error),
}

impl std::fmt::Display for BridgeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Extract(e) => write!(f, "manifest extraction failed: {e}"),
            Self::RemoteNotEnabled(url) => {
                write!(
                    f,
                    "reference is a URL ({url}); enable the `remote` feature to resolve it"
                )
            }
            #[cfg(feature = "remote")]
            Self::Fetch(msg) => write!(f, "remote manifest fetch failed: {msg}"),
            Self::C2pa(e) => write!(f, "c2pa validation error: {e}"),
        }
    }
}

impl std::error::Error for BridgeError {}

impl From<Error> for BridgeError {
    fn from(e: Error) -> Self {
        BridgeError::Extract(e)
    }
}

impl From<c2pa::Error> for BridgeError {
    fn from(e: c2pa::Error) -> Self {
        BridgeError::C2pa(e)
    }
}

/// Validate structured `text` against a C2PA Manifest Store the caller has
/// already resolved (from a `data:` URI or its own fetch).
///
/// Returns the [`c2pa::Reader`]; call [`Reader::validation_state`] or
/// [`Reader::validation_results`] for the verdict. The reader validates the
/// signature and trust chain, and the `c2pa.hash.data` binding over the raw
/// bytes of `text` (with the manifest block excluded by the assertion's
/// exclusion range).
pub fn validate_with_manifest(
    text: &str,
    c2pa_data: &[u8],
    format: &str,
) -> Result<Reader, BridgeError> {
    let reader = Reader::from_context(Context::default()).with_manifest_data_and_stream(
        c2pa_data,
        format,
        Cursor::new(text.as_bytes()),
    )?;
    Ok(reader)
}

/// Extract the reference from `text`, resolve it, and validate.
///
/// A `data:application/c2pa;base64,` reference is decoded inline. A URL
/// reference is fetched when the `remote` feature is enabled; otherwise this
/// returns [`BridgeError::RemoteNotEnabled`] and the caller should fetch the
/// bytes and use [`validate_with_manifest`].
pub fn validate(text: &str, format: &str) -> Result<Reader, BridgeError> {
    let extracted = extract_manifest(text)?;
    match classify_reference(&extracted.reference)? {
        Reference::Embedded(bytes) => validate_with_manifest(text, &bytes, format),
        Reference::Url(url) => {
            #[cfg(feature = "remote")]
            {
                let bytes = resolve_url(&url)?;
                validate_with_manifest(text, &bytes, format)
            }
            #[cfg(not(feature = "remote"))]
            {
                Err(BridgeError::RemoteNotEnabled(url))
            }
        }
    }
}

/// Fetch a remote C2PA Manifest Store over HTTP(S).
#[cfg(feature = "remote")]
pub fn resolve_url(url: &str) -> Result<Vec<u8>, BridgeError> {
    use std::io::Read;

    let resp = ureq::get(url)
        .call()
        .map_err(|e| BridgeError::Fetch(e.to_string()))?;
    let mut bytes = Vec::new();
    resp.into_reader()
        .take(64 * 1024 * 1024)
        .read_to_end(&mut bytes)
        .map_err(|e| BridgeError::Fetch(e.to_string()))?;
    Ok(bytes)
}
