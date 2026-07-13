// Copyright 2026 WritersLogic. All rights reserved.
// Licensed under the Apache License, Version 2.0 or the MIT license,
// at your option.

//! Python bindings for PyPI (via PyO3, built with maturin).
//!
//! Exposes the dependency-light core -- embed, extract, classify, and the hard
//! binding (compute/verify). Signature and trust validation are out of scope;
//! use the `c2pa` Python package for that.

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyBytes;

use crate::{embed, extract, hardbinding, Error};

fn to_py(e: Error) -> PyErr {
    PyValueError::new_err(e.to_string())
}

/// Embed a manifest block referencing an external manifest URL, as the first
/// line of the file.
#[pyfunction]
#[pyo3(signature = (text, url, comment_prefix, comment_suffix=None))]
fn embed_manifest_url(
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
#[pyfunction]
#[pyo3(signature = (text, manifest, comment_prefix, comment_suffix=None))]
fn embed_manifest_embedded(
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

/// Extract the manifest reference string from structured text.
#[pyfunction]
fn extract_reference(text: &str) -> PyResult<String> {
    Ok(extract::extract_manifest(text).map_err(to_py)?.reference)
}

/// A classified reference: `kind` is `"url"` or `"embedded"`; exactly one of
/// `url` / `data` is populated.
#[pyclass]
struct ClassifiedReference {
    kind: String,
    url: Option<String>,
    data: Option<Vec<u8>>,
}

#[pymethods]
impl ClassifiedReference {
    #[getter]
    fn kind(&self) -> &str {
        &self.kind
    }
    #[getter]
    fn url(&self) -> Option<&str> {
        self.url.as_deref()
    }
    #[getter]
    fn data<'py>(&self, py: Python<'py>) -> Option<Bound<'py, PyBytes>> {
        self.data.as_ref().map(|b| PyBytes::new(py, b))
    }
}

/// Classify and resolve a reference string: decode a `data:` URI to the
/// manifest bytes, or return an external URL.
#[pyfunction]
fn classify_reference(reference: &str) -> PyResult<ClassifiedReference> {
    match extract::classify_reference(reference).map_err(to_py)? {
        extract::Reference::Url(url) => Ok(ClassifiedReference {
            kind: "url".to_string(),
            url: Some(url),
            data: None,
        }),
        extract::Reference::Embedded(bytes) => Ok(ClassifiedReference {
            kind: "embedded".to_string(),
            url: None,
            data: Some(bytes),
        }),
    }
}

/// Compute the `c2pa.hash.data` exclusion range `(start, length)` covering the
/// manifest block.
#[pyfunction]
fn manifest_exclusion(text: &str) -> PyResult<(usize, usize)> {
    let ex = hardbinding::manifest_exclusion(text).map_err(to_py)?;
    Ok((ex.start, ex.length))
}

/// A computed structured-text `c2pa.hash.data` binding.
#[pyclass]
struct DataHash {
    inner: hardbinding::DataHash,
}

#[pymethods]
impl DataHash {
    #[getter]
    fn alg(&self) -> &str {
        &self.inner.alg
    }
    #[getter]
    fn hash<'py>(&self, py: Python<'py>) -> Bound<'py, PyBytes> {
        PyBytes::new(py, &self.inner.hash)
    }
    #[getter]
    fn exclusions(&self) -> Vec<(usize, usize)> {
        self.inner
            .exclusions
            .iter()
            .map(|e| (e.start, e.length))
            .collect()
    }
    /// The assertion serialised to `c2pa`-compatible JSON.
    fn to_json(&self) -> String {
        self.inner.to_json()
    }
}

/// Compute the hard binding for `text` with `alg` in (`"sha256"`, `"sha384"`,
/// `"sha512"`).
#[pyfunction]
fn compute_data_hash(text: &str, alg: &str) -> PyResult<DataHash> {
    let algorithm = hardbinding::Algorithm::from_id(alg).map_err(to_py)?;
    Ok(DataHash {
        inner: hardbinding::compute_data_hash(text, algorithm).map_err(to_py)?,
    })
}

/// Verify a hard binding against `text`. Returns `True` on a match, `False` on
/// a content mismatch, and raises on a malformed assertion or unsupported
/// algorithm.
#[pyfunction]
fn verify_data_hash(text: &str, data_hash: &DataHash) -> PyResult<bool> {
    match hardbinding::verify_data_hash(text, &data_hash.inner) {
        Ok(()) => Ok(true),
        Err(Error::HashMismatch) => Ok(false),
        Err(e) => Err(to_py(e)),
    }
}

#[pymodule]
fn c2pa_structured_text(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(embed_manifest_url, m)?)?;
    m.add_function(wrap_pyfunction!(embed_manifest_embedded, m)?)?;
    m.add_function(wrap_pyfunction!(extract_reference, m)?)?;
    m.add_function(wrap_pyfunction!(classify_reference, m)?)?;
    m.add_function(wrap_pyfunction!(manifest_exclusion, m)?)?;
    m.add_function(wrap_pyfunction!(compute_data_hash, m)?)?;
    m.add_function(wrap_pyfunction!(verify_data_hash, m)?)?;
    m.add_class::<ClassifiedReference>()?;
    m.add_class::<DataHash>()?;
    Ok(())
}
