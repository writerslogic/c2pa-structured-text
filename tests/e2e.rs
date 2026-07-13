// Copyright 2026 WritersLogic. All rights reserved.
// Licensed under the Apache License, Version 2.0 or the MIT license,
// at your option.

//! End-to-end: this crate embeds the block and computes the exclusion, c2pa-rs
//! signs a manifest whose `c2pa.hash.data` binds that exclusion, and this
//! crate's bridge validates it back through c2pa-rs.
//!
//! The manifest is signed as a raw C2PA store (format `"c2pa"`, an identity
//! composer) and validated as a sidecar against the text stream, so c2pa-rs's
//! raw-byte data hash is exercised directly against the block this crate embeds.

#![cfg(all(feature = "c2pa", feature = "hard-binding"))]

use std::io::Cursor;

use c2pa::{assertions::DataHash, create_signer, Builder, Context, HashRange, Signer, SigningAlg};

use c2pa_structured_text::{
    bridge::validate_with_manifest, embed_front_matter, embed_manifest, embed_manifest_at_end,
    hardbinding::manifest_exclusion, ManifestRef,
};

const URL: &str = "https://fabrikam.com/manifests/a1b2c3.c2pa";
// Signing format: c2pa-rs's identity composer, yielding a raw JUMBF store.
const STORE_FORMAT: &str = "c2pa";
// Validation format for the text stream (data hash is format-agnostic here).
const TEXT_FORMAT: &str = "text/plain";

fn signer() -> c2pa::BoxedSigner {
    create_signer::from_keys(
        include_bytes!("fixtures/certs/es256.pub"),
        include_bytes!("fixtures/certs/es256.pem"),
        SigningAlg::Es256,
        None,
    )
    .expect("build test signer")
}

/// Sign a sidecar C2PA Manifest Store whose `c2pa.hash.data` binds `embedded`
/// with the manifest block excluded, and return the raw JUMBF bytes.
fn sign_for(embedded: &str) -> Vec<u8> {
    let exclusion = manifest_exclusion(embedded).expect("exclusion");

    let mut dh = DataHash::new("structured text", "sha256");
    dh.add_exclusion(HashRange::new(
        exclusion.start as u64,
        exclusion.length as u64,
    ));
    dh.gen_hash_from_stream(&mut Cursor::new(embedded.as_bytes()))
        .expect("compute c2pa data hash");

    let mut builder = Builder::from_context(Context::default())
        .with_definition(
            r#"{"claim_generator_info":[{"name":"c2pa-structured-text","version":"0.1.1"}]}"#,
        )
        .expect("build manifest");

    let s = signer();
    builder
        .data_hashed_placeholder(s.reserve_size(), STORE_FORMAT)
        .expect("reserve placeholder");
    builder
        .sign_data_hashed_embeddable(s.as_ref(), &dh, STORE_FORMAT)
        .expect("sign data-hashed manifest")
}

fn has_code(reader: &c2pa::Reader, code: &str) -> bool {
    reader
        .validation_results()
        .and_then(|r| r.active_manifest())
        .map(|am| {
            am.success()
                .iter()
                .chain(am.failure().iter())
                .any(|s| s.code() == code)
        })
        .unwrap_or(false)
}

#[test]
fn embed_sign_validate_round_trip() {
    let source = "import os\n\nprint(os.getcwd())\n";
    let embedded = embed_manifest(source, ManifestRef::Url(URL), "#", None);
    let manifest = sign_for(&embedded);

    let reader = validate_with_manifest(&embedded, &manifest, TEXT_FORMAT).expect("validate");

    assert!(
        has_code(&reader, "assertion.dataHash.match"),
        "expected data hash match, got: {reader}"
    );
    assert!(!has_code(&reader, "assertion.dataHash.mismatch"));
}

#[test]
fn end_placement_round_trip() {
    // A shebang forces end-of-file placement; the exclusion covers the
    // preceding newline, which c2pa-rs must reproduce for the hash to match.
    let source = "#!/usr/bin/env bash\nset -euo pipefail\necho hi\n";
    let embedded = embed_manifest_at_end(source, ManifestRef::Url(URL), "#", None);
    let manifest = sign_for(&embedded);

    let reader = validate_with_manifest(&embedded, &manifest, TEXT_FORMAT).expect("validate");
    assert!(
        has_code(&reader, "assertion.dataHash.match"),
        "expected data hash match, got: {reader}"
    );
}

#[test]
fn front_matter_round_trip() {
    let source = "title: Report\nauthor: Q\n";
    let embedded = embed_front_matter(source, ManifestRef::Url(URL), "---");
    let manifest = sign_for(&embedded);

    let reader = validate_with_manifest(&embedded, &manifest, TEXT_FORMAT).expect("validate");
    assert!(
        has_code(&reader, "assertion.dataHash.match"),
        "expected data hash match, got: {reader}"
    );
}

#[test]
fn tampered_content_is_rejected() {
    let source = "SELECT id FROM accounts;\n";
    let embedded = embed_manifest(source, ManifestRef::Url(URL), "--", None);
    let manifest = sign_for(&embedded);

    // Flip covered bytes without changing length (outside the excluded block).
    let tampered = embedded.replace("accounts", "secrets_");
    assert_eq!(
        tampered.len(),
        embedded.len(),
        "tamper must preserve length"
    );

    let reader = validate_with_manifest(&tampered, &manifest, TEXT_FORMAT).expect("reader loads");
    assert!(
        has_code(&reader, "assertion.dataHash.mismatch"),
        "expected mismatch on tampered content, got: {reader}"
    );
}
