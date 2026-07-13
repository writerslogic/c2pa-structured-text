// Copyright 2026 WritersLogic. All rights reserved.
// Licensed under the Apache License, Version 2.0 or the MIT license,
// at your option.

//! Embedding a C2PA manifest block into structured text.
//!
//! The block is placed either at the beginning of the file (recommended), at
//! the end (when the first line is reserved by the host format, e.g. a shebang
//! or XML declaration), or inside a front matter section. Placement determines
//! the hard-binding exclusion range; see [`crate::hardbinding`].

use crate::codec;

const BEGIN: &str = "-----BEGIN C2PA MANIFEST-----";
const END: &str = "-----END C2PA MANIFEST-----";

/// A manifest reference to embed: either a URI to an external C2PA Manifest
/// Store (preferred) or the store itself, which is encoded as a
/// `data:application/c2pa;base64,` URI.
pub enum ManifestRef<'a> {
    Url(&'a str),
    Embedded(&'a [u8]),
}

impl ManifestRef<'_> {
    fn render(&self) -> String {
        match self {
            ManifestRef::Url(url) => (*url).to_string(),
            ManifestRef::Embedded(bytes) => {
                format!("data:application/c2pa;base64,{}", codec::encode(bytes))
            }
        }
    }
}

fn single_line(reference: &str, comment_prefix: &str, comment_suffix: Option<&str>) -> String {
    let suffix = comment_suffix.unwrap_or("");
    format!("{comment_prefix} {BEGIN} {reference} {END} {suffix}")
        .trim_end()
        .to_string()
}

/// Embed a manifest block as the first line of the file using single-line
/// comment syntax. This is the recommended placement.
pub fn embed_manifest(
    text: &str,
    manifest: ManifestRef<'_>,
    comment_prefix: &str,
    comment_suffix: Option<&str>,
) -> String {
    let line = single_line(&manifest.render(), comment_prefix, comment_suffix);
    format!("{line}\n{text}")
}

/// Embed a manifest block as the last line of the file. Use this when the first
/// line is reserved by the host format (a shebang `#!/...` or an XML
/// declaration `<?xml ...?>`), so the `-----END C2PA MANIFEST-----` delimiter
/// appears on the final line as the specification requires.
///
/// The block is separated from preceding content by a single newline; if the
/// text does not already end in a line terminator one is added first.
pub fn embed_manifest_at_end(
    text: &str,
    manifest: ManifestRef<'_>,
    comment_prefix: &str,
    comment_suffix: Option<&str>,
) -> String {
    let line = single_line(&manifest.render(), comment_prefix, comment_suffix);
    if text.is_empty() {
        return line;
    }
    if text.ends_with('\n') {
        format!("{text}{line}")
    } else {
        format!("{text}\n{line}")
    }
}

/// Embed a manifest block in multi-line front matter form. `fm_delim` is the
/// host format's front matter fence (`---` for YAML, `+++` for TOML).
///
/// If `text` already opens with `fm_delim` on its first line the C2PA block is
/// inserted at the top of that existing front matter; otherwise a new front
/// matter section containing only the block is prepended.
pub fn embed_front_matter(text: &str, manifest: ManifestRef<'_>, fm_delim: &str) -> String {
    let reference = manifest.render();
    let block = format!("{BEGIN}\n{reference}\n{END}");

    let opening = format!("{fm_delim}\n");
    if let Some(rest) = text.strip_prefix(&opening) {
        format!("{opening}{block}\n{rest}")
    } else {
        format!("{fm_delim}\n{block}\n{fm_delim}\n{text}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embed_url_python() {
        let text = "print('hello')\n";
        let result = embed_manifest(
            text,
            ManifestRef::Url("https://example.com/m.c2pa"),
            "#",
            None,
        );
        assert!(result.starts_with("# -----BEGIN C2PA MANIFEST-----"));
        assert!(result.contains("https://example.com/m.c2pa"));
        assert!(result.contains("-----END C2PA MANIFEST-----"));
        assert!(result.ends_with("print('hello')\n"));
    }

    #[test]
    fn embed_url_css() {
        let result = embed_manifest(
            "body {}",
            ManifestRef::Url("https://example.com/m.c2pa"),
            "/*",
            Some("*/"),
        );
        assert!(result.starts_with("/* -----BEGIN C2PA MANIFEST-----"));
        assert!(result.contains("-----END C2PA MANIFEST----- */"));
    }

    #[test]
    fn embed_data_uri() {
        let bytes = b"test manifest";
        let result = embed_manifest("content", ManifestRef::Embedded(bytes), "#", None);
        assert!(result.contains("data:application/c2pa;base64,"));
    }

    #[test]
    fn embed_at_end_after_shebang() {
        let text = "#!/usr/bin/env python3\nprint('hi')\n";
        let result = embed_manifest_at_end(
            text,
            ManifestRef::Url("https://example.com/m.c2pa"),
            "#",
            None,
        );
        assert!(result.starts_with("#!/usr/bin/env python3"));
        assert!(result.ends_with("-----END C2PA MANIFEST-----"));
    }

    #[test]
    fn embed_at_end_adds_newline_separator() {
        let result =
            embed_manifest_at_end("no trailing newline", ManifestRef::Url("u"), "//", None);
        assert_eq!(
            result,
            "no trailing newline\n// -----BEGIN C2PA MANIFEST----- u -----END C2PA MANIFEST-----"
        );
    }

    #[test]
    fn embed_front_matter_into_existing() {
        let text = "---\ntitle: doc\n---\nbody\n";
        let result =
            embed_front_matter(text, ManifestRef::Url("https://example.com/m.c2pa"), "---");
        assert!(result.starts_with("---\n-----BEGIN C2PA MANIFEST-----\n"));
        assert!(result.contains("\ntitle: doc\n"));
    }

    #[test]
    fn embed_front_matter_creates_section() {
        let result = embed_front_matter("# Heading\n", ManifestRef::Url("u"), "---");
        assert!(result.starts_with(
            "---\n-----BEGIN C2PA MANIFEST-----\nu\n-----END C2PA MANIFEST-----\n---\n"
        ));
    }
}
