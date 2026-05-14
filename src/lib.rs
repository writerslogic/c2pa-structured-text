// Copyright 2026 WritersLogic. All rights reserved.
// Licensed under the Apache License, Version 2.0 or the MIT license,
// at your option.

//! C2PA manifest embedding for structured text formats.
//!
//! Implements the Structured Text Embedding section of the C2PA Technical
//! Specification, which defines how to associate a C2PA Manifest Store with
//! source code, configuration files, markup, and other text formats that
//! support comment syntax or front matter conventions.
//!
//! The manifest block uses fixed ASCII armour-style delimiters:
//! `-----BEGIN C2PA MANIFEST-----` and `-----END C2PA MANIFEST-----`

mod embed;
mod error;
mod extract;

pub use embed::{embed_manifest, ManifestRef};
pub use error::Error;
pub use extract::{extract_manifest, ExtractionResult};
