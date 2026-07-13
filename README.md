<p align="center">
  <h1 align="center">c2pa-structured-text</h1>
  <p align="center">C2PA manifest embedding, hard binding, and validation for structured text formats</p>
</p>

<p align="center">
  <a href="https://github.com/writerslogic/c2pa-structured-text/actions/workflows/ci.yml"><img src="https://github.com/writerslogic/c2pa-structured-text/actions/workflows/ci.yml/badge.svg" alt="CI"></a>
  <a href="https://crates.io/crates/c2pa-structured-text"><img src="https://img.shields.io/crates/v/c2pa-structured-text.svg" alt="crates.io"></a>
  <a href="https://docs.rs/c2pa-structured-text"><img src="https://docs.rs/c2pa-structured-text/badge.svg" alt="docs.rs"></a>
  <a href="#license"><img src="https://img.shields.io/crates/l/c2pa-structured-text.svg" alt="License"></a>
</p>

## Overview

Implements the **Embedding Manifests into Structured Text** section of the [C2PA Technical Specification](https://spec.c2pa.org/specifications/specifications/2.4/specs/C2PA_Specification.html#_embedding_manifests_into_structured_text), which associates a C2PA Manifest Store with source code, configuration files, markup, and other text formats that support comment syntax or front matter conventions.

The manifest block uses fixed ASCII armour-style delimiters modelled on [RFC 4880](https://www.rfc-editor.org/rfc/rfc4880#section-6.2):

```
-----BEGIN C2PA MANIFEST----- <reference> -----END C2PA MANIFEST-----
```

This crate owns three things:

1. **Embed / Extract** — place a reference or an inline manifest as a comment or front matter block, and locate and resolve it again.
2. **Hard binding** — define and compute the exact `c2pa.hash.data` coverage for structured text, and verify it.
3. **A validation bridge** to [`c2pa-rs`](https://crates.io/crates/c2pa) for signature, trust, and assertion validation — which this crate does *not* reimplement.

> This crate is not certified or conformance-tested by the C2PA. It implements the structured-text embedding and hard binding as specified, and delegates cryptographic validation to `c2pa-rs`.

## Quick Start

```toml
[dependencies]
c2pa-structured-text = "0.1"
```

### Embed a manifest reference

```rust
use c2pa_structured_text::{embed_manifest, ManifestRef};

let signed = embed_manifest(
    "print('hello')\n",
    ManifestRef::Url("https://example.com/manifests/abc.c2pa"),
    "#",   // comment prefix
    None,  // no comment suffix
);
// # -----BEGIN C2PA MANIFEST----- https://example.com/manifests/abc.c2pa -----END C2PA MANIFEST-----
// print('hello')
```

`embed_manifest_at_end` places the block on the last line (for files whose first line is reserved, e.g. a shebang or XML declaration), and `embed_front_matter` writes the multi-line form inside YAML/TOML front matter.

### Extract a manifest reference

```rust
use c2pa_structured_text::{extract_manifest, classify_reference, Reference};

let text = "# -----BEGIN C2PA MANIFEST----- https://example.com/m.c2pa -----END C2PA MANIFEST-----
print('hello')
";
let result = extract_manifest(text).unwrap();
assert_eq!(result.reference, "https://example.com/m.c2pa");

// A `data:application/c2pa;base64,` reference decodes to the manifest bytes;
// anything else is treated as an external URI.
match classify_reference(&result.reference).unwrap() {
    Reference::Url(url) => { /* fetch it */ }
    Reference::Embedded(bytes) => { /* raw JUMBF manifest store */ }
}
```

## The Hard Binding

A structured-text manifest is bound with a `c2pa.hash.data` assertion carrying a **single exclusion range covering the entire manifest block**. The hash is computed over the **raw bytes** of the file with that range removed.

Unlike the Unicode Variation Selector method for *unstructured* text, this binding applies **no Unicode normalization**: structured text files are byte-stable on disk, and normalizing to NFC would create false mismatches for files that legitimately contain NFD content. Files must be read in binary mode, preserving exact line terminators; bare CR line endings are unsupported.

```rust
# #[cfg(feature = "hard-binding")] {
use c2pa_structured_text::hardbinding::{compute_data_hash, verify_data_hash, Algorithm};

let signed = c2pa_structured_text::embed_manifest(
    "print('hello')\n",
    c2pa_structured_text::ManifestRef::Url("https://example.com/m.c2pa"),
    "#",
    None,
);
let data_hash = compute_data_hash(&signed, Algorithm::Sha256).unwrap();
verify_data_hash(&signed, &data_hash).unwrap();
# }
```

The exclusion-range and covered-byte primitives (`manifest_exclusion`, `hashed_bytes`) are always available and dependency-free; `compute_data_hash` / `verify_data_hash` require the `hard-binding` feature (which pulls `sha2`).

### Fragility — and the soft-binding recovery path

This is a **byte-exact** binding, and it is meant to be. Any change to the covered bytes — reformatting, re-indentation, transcoding, or an LF↔CRLF conversion outside the block — breaks it. Where durability across such transformations matters, pair it with the perceptual soft binding in [c2pa-text-binding](https://github.com/writerslogic/c2pa-text-binding), which re-associates transformed content with its provenance after the hard binding is lost. Do not treat the structured-text hard binding as robust to editing.

## Validating with c2pa-rs

Enable the `c2pa` feature to validate the signature, trust chain, and hard binding via `c2pa-rs`. This crate extracts and resolves the reference; `c2pa-rs` does the cryptography.

```rust,ignore
use c2pa_structured_text::bridge;

// Inline (data:) references are decoded automatically; URL references are
// fetched with the `remote` feature (or resolve them yourself and call
// `bridge::validate_with_manifest`).
let reader = bridge::validate(&signed, bridge::DEFAULT_FORMAT)?;
println!("{:?}", reader.validation_state());
```

## Features

| Feature | Adds | Pulls |
|---|---|---|
| *(none)* | embed, extract, exclusion-range and covered-byte primitives | — |
| `hard-binding` | `compute_data_hash` / `verify_data_hash` (SHA2-256/384/512) | `sha2` |
| `c2pa` | the `bridge` to `c2pa-rs` for signature/trust/assertion validation | `c2pa` |
| `remote` | HTTP(S) resolution of URL references in the bridge | `c2pa`, `ureq` |

No feature is enabled by default; the core API has no dependencies.

## Supported Formats

Any text format with a comment syntax or front matter convention:

| Comment Style | Formats | Example |
|---|---|---|
| `#` | Python, Ruby, Shell, YAML, TOML | `# -----BEGIN C2PA MANIFEST----- ... -----END C2PA MANIFEST-----` |
| `//` | JavaScript, TypeScript, Go, Rust, C++ | `// -----BEGIN C2PA MANIFEST----- ... -----END C2PA MANIFEST-----` |
| `--` | SQL, Lua, Haskell | `-- -----BEGIN C2PA MANIFEST----- ... -----END C2PA MANIFEST-----` |
| `/* */` | CSS, C, Java | `/* -----BEGIN C2PA MANIFEST----- ... -----END C2PA MANIFEST----- */` |
| `<!-- -->` | Markdown, XML (non-HTML) | `<!-- -----BEGIN C2PA MANIFEST----- ... -----END C2PA MANIFEST----- -->` |
| Front matter | Markdown (YAML), TOML | Multi-line form between front matter delimiters |

> **WebVTT** is structured text per the specification, but it is owned by the dedicated [c2pa-vtt](https://github.com/writerslogic/c2pa-vtt) crate, not this one. `c2pa-vtt` places the `NOTE` block immediately after the `WEBVTT` header, where it survives HLS/DASH segmentation. Use `c2pa-vtt` for `.vtt` files.
>
> **HTML** and **SVG** have their own C2PA embedding methods and are out of scope here.

## Related Crates

| Crate | Description |
|---|---|
| [c2pa-vtt](https://github.com/writerslogic/c2pa-vtt) | **Canonical owner of WebVTT** captions/subtitles (streaming-safe placement) |
| [c2pa-text-binding](https://github.com/writerslogic/c2pa-text-binding) | Soft binding and content fingerprinting for text assets (recovery path) |
| [c2pa-text](https://crates.io/crates/c2pa-text) | Unstructured text embedding via Unicode Variation Selectors |
| [c2pa-rs](https://crates.io/crates/c2pa) | Official C2PA SDK |

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or [MIT License](LICENSE-MIT) at your option.

Built by [WritersLogic](https://writerslogic.com)