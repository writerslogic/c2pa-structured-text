// Copyright 2026 WritersLogic. All rights reserved.
// Licensed under the Apache License, Version 2.0 or the MIT license,
// at your option.

//! WebAssembly bindings for npm (via `wasm-bindgen`).
//!
//! Exposes the dependency-light core -- embed, extract, classify, and the hard
//! binding (compute/verify) -- to JavaScript. Signature and trust validation
//! are intentionally out of scope here; use `c2pa-js` for that, exactly as the
//! Rust `bridge` defers to `c2pa-rs`.

use wasm_bindgen::prelude::*;

use crate::{embed, extract, hardbinding, Error};

fn to_js(e: Error) -> JsError {
    JsError::new(&e.to_string())
}

/// Embed a manifest block referencing an external manifest URL, as the first
/// line of the file.
#[wasm_bindgen(js_name = embedManifestUrl)]
pub fn embed_manifest_url(
    text: &str,
    url: &str,
    comment_prefix: &str,
    comment_suffix: Option<String>,
) -> String {
    embed::embed_manifest(
        text,
        embed::ManifestRef::Url(url),
        comment_prefix,
        comment_suffix.as_deref(),
    )
}

/// Embed a manifest block carrying the manifest store inline as a
/// `data:application/c2pa;base64,` URI.
#[wasm_bindgen(js_name = embedManifestEmbedded)]
pub fn embed_manifest_embedded(
    text: &str,
    manifest: &[u8],
    comment_prefix: &str,
    comment_suffix: Option<String>,
) -> String {
    embed::embed_manifest(
        text,
        embed::ManifestRef::Embedded(manifest),
        comment_prefix,
        comment_suffix.as_deref(),
    )
}

/// Extract the manifest reference string from structured text. Throws if no
/// single manifest block is present.
#[wasm_bindgen(js_name = extractReference)]
pub fn extract_reference(text: &str) -> Result<String, JsError> {
    Ok(extract::extract_manifest(text).map_err(to_js)?.reference)
}

/// A classified reference: `kind` is `"url"` or `"embedded"`; exactly one of
/// `url` / `bytes` is populated.
#[wasm_bindgen]
pub struct ClassifiedReference {
    kind: String,
    url: Option<String>,
    bytes: Option<Vec<u8>>,
}

#[wasm_bindgen]
impl ClassifiedReference {
    #[wasm_bindgen(getter)]
    pub fn kind(&self) -> String {
        self.kind.clone()
    }
    #[wasm_bindgen(getter)]
    pub fn url(&self) -> Option<String> {
        self.url.clone()
    }
    #[wasm_bindgen(getter)]
    pub fn bytes(&self) -> Option<Vec<u8>> {
        self.bytes.clone()
    }
}

/// Classify and resolve a reference string: decode a `data:` URI to the
/// manifest bytes, or return an external URL.
#[wasm_bindgen(js_name = classifyReference)]
pub fn classify_reference(reference: &str) -> Result<ClassifiedReference, JsError> {
    match extract::classify_reference(reference).map_err(to_js)? {
        extract::Reference::Url(url) => Ok(ClassifiedReference {
            kind: "url".to_string(),
            url: Some(url),
            bytes: None,
        }),
        extract::Reference::Embedded(bytes) => Ok(ClassifiedReference {
            kind: "embedded".to_string(),
            url: None,
            bytes: Some(bytes),
        }),
    }
}

/// The hard-binding exclusion range over the manifest block.
#[wasm_bindgen]
pub struct Exclusion {
    #[wasm_bindgen(readonly)]
    pub start: usize,
    #[wasm_bindgen(readonly)]
    pub length: usize,
}

/// Compute the `c2pa.hash.data` exclusion range covering the manifest block.
#[wasm_bindgen(js_name = manifestExclusion)]
pub fn manifest_exclusion(text: &str) -> Result<Exclusion, JsError> {
    let ex = hardbinding::manifest_exclusion(text).map_err(to_js)?;
    Ok(Exclusion {
        start: ex.start,
        length: ex.length,
    })
}

/// A computed structured-text `c2pa.hash.data` binding.
#[wasm_bindgen]
pub struct DataHash {
    inner: hardbinding::DataHash,
}

#[wasm_bindgen]
impl DataHash {
    #[wasm_bindgen(getter)]
    pub fn alg(&self) -> String {
        self.inner.alg.clone()
    }
    #[wasm_bindgen(getter)]
    pub fn hash(&self) -> Vec<u8> {
        self.inner.hash.clone()
    }
    /// The assertion serialised to `c2pa-rs`-compatible JSON.
    #[wasm_bindgen(js_name = toJson)]
    pub fn to_json(&self) -> String {
        self.inner.to_json()
    }
}

/// Compute the hard binding for `text` with `alg` in (`"sha256"`, `"sha384"`,
/// `"sha512"`).
#[wasm_bindgen(js_name = computeDataHash)]
pub fn compute_data_hash(text: &str, alg: &str) -> Result<DataHash, JsError> {
    let algorithm = hardbinding::Algorithm::from_id(alg).map_err(to_js)?;
    Ok(DataHash {
        inner: hardbinding::compute_data_hash(text, algorithm).map_err(to_js)?,
    })
}

/// Verify a hard binding against `text`. Returns `true` on a match, `false` on
/// a content mismatch, and throws on a malformed assertion or unsupported
/// algorithm.
#[wasm_bindgen(js_name = verifyDataHash)]
pub fn verify_data_hash(text: &str, data_hash: &DataHash) -> Result<bool, JsError> {
    match hardbinding::verify_data_hash(text, &data_hash.inner) {
        Ok(()) => Ok(true),
        Err(Error::HashMismatch) => Ok(false),
        Err(e) => Err(to_js(e)),
    }
}
