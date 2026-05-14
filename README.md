<p align="center">
  <h1 align="center">c2pa-structured-text</h1>
  <p align="center">C2PA manifest embedding for structured text formats</p>
</p>

<p align="center">
  <a href="https://crates.io/crates/c2pa-structured-text"><img src="https://img.shields.io/crates/v/c2pa-structured-text.svg" alt="crates.io"></a>
  <a href="https://docs.rs/c2pa-structured-text"><img src="https://docs.rs/c2pa-structured-text/badge.svg" alt="docs.rs"></a>
  <a href="#license"><img src="https://img.shields.io/crates/l/c2pa-structured-text.svg" alt="License"></a>
</p>

## Overview

Implements the **Structured Text Embedding** section of the [C2PA Technical Specification](https://c2pa.org/specifications/), which defines how to associate a C2PA Manifest Store with source code, configuration files, markup, and other text formats that support comment syntax or front matter conventions.

The manifest block uses fixed ASCII armour-style delimiters modelled on [RFC 4880](https://www.rfc-editor.org/rfc/rfc4880#section-6.2):

```
-----BEGIN C2PA MANIFEST----- <reference> -----END C2PA MANIFEST-----
```

## Quick Start

```toml
[dependencies]
c2pa-structured-text = "0.1"
```

### Embed a manifest reference

```rust
use c2pa_structured_text::{embed_manifest, ManifestRef};

let text = "print('hello')\n";
let signed = embed_manifest(
    text,
    ManifestRef::Url("https://example.com/manifests/abc.c2pa"),
    "#",       // comment prefix
    None,      // no comment suffix
);
// Result:
// # -----BEGIN C2PA MANIFEST----- https://example.com/manifests/abc.c2pa -----END C2PA MANIFEST-----
// print('hello')
```

### Extract a manifest reference

```rust
use c2pa_structured_text::extract_manifest;

let text = "# -----BEGIN C2PA MANIFEST----- https://example.com/m.c2pa -----END C2PA MANIFEST-----\nprint('hello')\n";
let result = extract_manifest(text).unwrap();
assert_eq!(result.reference, "https://example.com/m.c2pa");
```

## Supported Formats

Any text format with a comment syntax or front matter convention:

| Comment Style | Formats | Example |
|---|---|---|
| `#` | Python, Ruby, Shell, YAML, TOML | `# -----BEGIN C2PA MANIFEST----- ... -----END C2PA MANIFEST-----` |
| `//` | JavaScript, TypeScript, Go, Rust, C++ | `// -----BEGIN C2PA MANIFEST----- ... -----END C2PA MANIFEST-----` |
| `--` | SQL, Lua, Haskell | `-- -----BEGIN C2PA MANIFEST----- ... -----END C2PA MANIFEST-----` |
| `/* */` | CSS, C, Java | `/* -----BEGIN C2PA MANIFEST----- ... -----END C2PA MANIFEST----- */` |
| `<!-- -->` | HTML, XML, Markdown | `<!-- -----BEGIN C2PA MANIFEST----- ... -----END C2PA MANIFEST----- -->` |
| `NOTE` | WebVTT | `NOTE -----BEGIN C2PA MANIFEST----- ... -----END C2PA MANIFEST-----` |
| Front matter | Markdown (YAML), TOML | Multi-line form between front matter delimiters |

## Related Crates

| Crate | Description |
|---|---|
| [c2pa-text-binding](https://github.com/writerslogic/c2pa-text-binding) | Soft binding and content fingerprinting for text assets |
| [c2pa-text](https://crates.io/crates/c2pa-text) | Unstructured text embedding via Unicode Variation Selectors |
| [c2pa-rs](https://crates.io/crates/c2pa) | Official C2PA SDK |

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or [MIT License](LICENSE-MIT) at your option.

Built by [WritersLogic](https://writerslogic.com)
