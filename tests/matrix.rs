// Copyright 2026 WritersLogic. All rights reserved.
// Licensed under the Apache License, Version 2.0 or the MIT license,
// at your option.

//! Format matrix: embed -> extract -> classify -> compute -> verify across
//! every comment and front matter convention in the README, using real-world
//! sample content for each host language.

#![cfg(feature = "hard-binding")]

use c2pa_structured_text::{
    classify_reference, embed_front_matter, embed_manifest, embed_manifest_at_end,
    extract_manifest,
    hardbinding::{compute_data_hash, manifest_exclusion, verify_data_hash, Algorithm},
    ManifestRef, Reference,
};

const URL: &str = "https://fabrikam.com/manifests/a1b2c3.c2pa";

/// Single-line comment styles: (prefix, suffix, sample source).
const SINGLE_LINE: &[(&str, Option<&str>, &str)] = &[
    (
        "#",
        None,
        "import sys\n\ndef main():\n    print(sys.argv)\n",
    ),
    (
        "//",
        None,
        "export function add(a: number, b: number) {\n  return a + b;\n}\n",
    ),
    (
        "--",
        None,
        "SELECT id, name FROM users\nWHERE active = true\nORDER BY name;\n",
    ),
    (
        "/*",
        Some("*/"),
        "body {\n  margin: 0;\n  padding: 1rem;\n}\n",
    ),
    (
        "<!--",
        Some("-->"),
        "# Heading\n\nA paragraph of Markdown prose.\n",
    ),
];

fn assert_round_trip(embedded: &str, expected_covered: &[u8]) {
    // Extraction recovers the reference.
    let extracted = extract_manifest(embedded).unwrap();
    assert_eq!(extracted.reference, URL);
    assert_eq!(
        classify_reference(&extracted.reference).unwrap(),
        Reference::Url(URL.to_string())
    );

    // The covered bytes are exactly the original content region.
    let exclusion = manifest_exclusion(embedded).unwrap();
    assert_eq!(exclusion.start, extracted.offset);

    // Compute then verify succeeds for every algorithm.
    for alg in [Algorithm::Sha256, Algorithm::Sha384, Algorithm::Sha512] {
        let dh = compute_data_hash(embedded, alg).unwrap();
        assert_eq!(dh.alg, alg.id());
        assert_eq!(dh.exclusions, vec![exclusion]);
        verify_data_hash(embedded, &dh).unwrap();

        // A one-byte change to the covered content breaks the binding.
        let tampered = tamper_covered(embedded, expected_covered);
        assert!(verify_data_hash(&tampered, &dh).is_err());
    }
}

/// Flip a byte inside the covered content (never inside the excluded block).
fn tamper_covered(embedded: &str, covered: &[u8]) -> String {
    assert!(!covered.is_empty(), "test content must be non-empty");
    let needle = &covered[..covered.len().min(4)];
    let pos = embedded
        .as_bytes()
        .windows(needle.len())
        .position(|w| w == needle)
        .unwrap();
    let mut bytes = embedded.as_bytes().to_vec();
    bytes[pos] ^= 0x20;
    String::from_utf8(bytes).unwrap()
}

#[test]
fn single_line_beginning() {
    for (prefix, suffix, source) in SINGLE_LINE {
        let embedded = embed_manifest(source, ManifestRef::Url(URL), prefix, *suffix);
        assert_round_trip(&embedded, source.as_bytes());
    }
}

#[test]
fn single_line_end_after_reserved_line() {
    // Shebang and XML declaration force end-of-file placement.
    let script = "#!/usr/bin/env bash\nset -euo pipefail\necho hello\n";
    let embedded = embed_manifest_at_end(script, ManifestRef::Url(URL), "#", None);
    let extracted = extract_manifest(&embedded).unwrap();
    assert_eq!(extracted.reference, URL);
    let dh = compute_data_hash(&embedded, Algorithm::Sha256).unwrap();
    verify_data_hash(&embedded, &dh).unwrap();

    let xml = "<?xml version=\"1.0\"?>\n<root><child/></root>\n";
    let embedded = embed_manifest_at_end(xml, ManifestRef::Url(URL), "<!--", Some("-->"));
    assert!(embedded
        .trim_end()
        .ends_with("-----END C2PA MANIFEST----- -->"));
    let dh = compute_data_hash(&embedded, Algorithm::Sha256).unwrap();
    verify_data_hash(&embedded, &dh).unwrap();
}

#[test]
fn front_matter_multi_line() {
    let markdown = "title: My Document\nauthor: Jane\n";
    let embedded = embed_front_matter(markdown, ManifestRef::Url(URL), "---");
    assert_round_trip(&embedded, markdown.as_bytes());

    // TOML front matter fence.
    let toml_fm = "title = \"doc\"\n";
    let embedded = embed_front_matter(toml_fm, ManifestRef::Url(URL), "+++");
    let dh = compute_data_hash(&embedded, Algorithm::Sha256).unwrap();
    verify_data_hash(&embedded, &dh).unwrap();
}

#[test]
fn embedded_data_uri_round_trips() {
    // An inline manifest store as a data: URI decodes back to the same bytes,
    // and the binding is independent of the (excluded) manifest content.
    let manifest_bytes = b"\x00\x01\x02 pretend JUMBF manifest store \xfe\xff";
    let source = "const x = 1;\n";
    let embedded = embed_manifest(source, ManifestRef::Embedded(manifest_bytes), "//", None);

    let extracted = extract_manifest(&embedded).unwrap();
    match classify_reference(&extracted.reference).unwrap() {
        Reference::Embedded(bytes) => assert_eq!(bytes, manifest_bytes),
        other => panic!("expected embedded manifest, got {other:?}"),
    }

    let dh = compute_data_hash(&embedded, Algorithm::Sha256).unwrap();
    verify_data_hash(&embedded, &dh).unwrap();
    // The covered region is the original source, regardless of manifest size.
    assert_eq!(
        c2pa_structured_text::hardbinding::hashed_bytes(&embedded).unwrap(),
        source.as_bytes()
    );
}

#[test]
fn crlf_content_binds() {
    let source = "line one\r\nline two\r\n";
    let embedded = embed_manifest(source, ManifestRef::Url(URL), "//", None);
    // Re-embed produced an LF after the block; splice CRLF to keep it uniform.
    let embedded = embedded.replacen(
        "-----END C2PA MANIFEST-----\n",
        "-----END C2PA MANIFEST-----\r\n",
        1,
    );
    let dh = compute_data_hash(&embedded, Algorithm::Sha256).unwrap();
    verify_data_hash(&embedded, &dh).unwrap();
    assert_eq!(
        c2pa_structured_text::hardbinding::hashed_bytes(&embedded).unwrap(),
        source.as_bytes()
    );
}
