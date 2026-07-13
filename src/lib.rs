// Copyright 2026 WritersLogic. All rights reserved.
// Licensed under the Apache License, Version 2.0 or the MIT license,
// at your option.

//! C2PA manifest embedding, hard binding, and validation for structured text.
//!
//! Implements the *Embedding Manifests into Structured Text* section of the
//! C2PA Technical Specification, which associates a C2PA Manifest Store with
//! source code, configuration files, markup, and other text formats that
//! support comment syntax or front matter conventions.
//!
//! The manifest block uses fixed ASCII armour-style delimiters:
//! `-----BEGIN C2PA MANIFEST-----` and `-----END C2PA MANIFEST-----`.
//!
//! # Scope
//!
//! This crate owns three things:
//!
//! - **Embed** ([`embed_manifest`], [`embed_manifest_at_end`],
//!   [`embed_front_matter`]) a reference or an inline manifest as a comment or
//!   front matter block.
//! - **Extract** ([`extract_manifest`], [`classify_reference`]) the block and
//!   resolve its reference.
//! - The **hard binding** ([`hardbinding`]): the exact `c2pa.hash.data`
//!   coverage over the normalized-free raw byte stream, with the manifest block
//!   excluded, plus compute and verify.
//!
//! Signature verification, certificate trust, and assertion validation are
//! **not** implemented here; the [`bridge`] (feature `c2pa`) delegates them to
//! `c2pa-rs`.
//!
//! # Features
//!
//! - `hard-binding` — concrete SHA2-256/384/512 [`hardbinding::compute_data_hash`]
//!   and [`hardbinding::verify_data_hash`] (pulls `sha2`). The exclusion-range
//!   and covered-byte primitives ([`hardbinding::manifest_exclusion`],
//!   [`hardbinding::hashed_bytes`]) are always available and dependency-free.
//! - `c2pa` — the [`bridge`] to `c2pa-rs` for signature/trust/assertion validation.
//! - `remote` — HTTP(S) resolution of URL references in the bridge (pulls `ureq`).
//! - `wasm` — JS/WASM bindings for the npm package (pulls `wasm-bindgen`).
//! - `python` — Python bindings for the PyPI wheel (pulls `pyo3`, built with maturin).
//!
//! No feature is enabled by default; the core embed/extract/binding-range API
//! has no dependencies.

mod codec;
mod embed;
mod error;
mod extract;
pub mod hardbinding;

#[cfg(feature = "c2pa")]
pub mod bridge;

#[cfg(feature = "wasm")]
mod wasm;

#[cfg(feature = "python")]
mod python;

pub use embed::{embed_front_matter, embed_manifest, embed_manifest_at_end, ManifestRef};
pub use error::Error;
pub use extract::{classify_reference, extract_manifest, ExtractionResult, Reference};
