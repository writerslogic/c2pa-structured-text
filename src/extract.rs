// Copyright 2026 WritersLogic. All rights reserved.
// Licensed under the Apache License, Version 2.0 or the MIT license,
// at your option.

//! Locating and extracting the manifest block, and classifying its reference.

use crate::codec;
use crate::error::Error;

/// The opening ASCII armour delimiter for a C2PA manifest block.
pub const BEGIN: &str = "-----BEGIN C2PA MANIFEST-----";
/// The closing ASCII armour delimiter for a C2PA manifest block.
pub const END: &str = "-----END C2PA MANIFEST-----";
/// The URI prefix marking an inline (embedded) C2PA Manifest Store.
pub const DATA_URI_PREFIX: &str = "data:application/c2pa;base64,";

/// The located manifest block: the reference between the delimiters and the
/// byte span of the block's line(s) within the file.
///
/// `line_start` is the offset of the first byte of the line carrying the
/// `-----BEGIN C2PA MANIFEST-----` delimiter (including any host comment
/// prefix). `line_end` is the offset one past the trailing line terminator of
/// the line carrying `-----END C2PA MANIFEST-----`, or the end of the file if
/// that line has no terminator. For front matter, this spans only the delimiter
/// lines, never the surrounding `---`/`+++` fences.
pub(crate) struct Block {
    pub reference: String,
    pub line_start: usize,
    pub line_end: usize,
}

pub(crate) fn locate_block(text: &str) -> Result<Block, Error> {
    let bytes = text.as_bytes();
    reject_bare_cr(bytes)?;

    let begin_pos = find_delimiter(bytes, BEGIN).ok_or(Error::NotFound)?;
    let after_begin = begin_pos + BEGIN.len();

    let end_pos = find_delimiter(&bytes[after_begin..], END)
        .map(|pos| after_begin + pos)
        .ok_or(Error::NotFound)?;

    // BEGIN and END must each appear on their own line (front-matter form) or
    // together on one line with the reference between them (single-line form);
    // the spec requires one of these two shapes. An unconstrained scan would
    // let END be reached from anywhere later in the file, folding unrelated
    // file content into both the "reference" and the hash-exclusion range that
    // this block's span becomes -- content inside that range is excluded from
    // the hard binding and can be tampered with undetected. Both legal shapes
    // share one property: strip at most one leading and one trailing newline
    // from the raw span between the delimiters, and nothing else may remain on
    // its own line -- i.e. the reference itself must be a single line.
    let raw_between = &text[after_begin..end_pos];
    let trimmed_once = raw_between
        .strip_prefix("\r\n")
        .or_else(|| raw_between.strip_prefix('\n'))
        .unwrap_or(raw_between);
    let trimmed_once = trimmed_once
        .strip_suffix("\r\n")
        .or_else(|| trimmed_once.strip_suffix('\n'))
        .unwrap_or(trimmed_once);
    if trimmed_once.contains('\n') {
        return Err(Error::NotFound);
    }
    // A second BEGIN before this block's own END (same line or otherwise) means
    // two overlapping/nested blocks, not one legitimate reference that happens
    // to mention the word BEGIN -- reject it the same way a second block found
    // after END already is, rather than silently absorbing it into the
    // reference.
    if find_delimiter(raw_between.as_bytes(), BEGIN).is_some() {
        return Err(Error::MultipleBlocks);
    }

    let after_end = end_pos + END.len();
    if find_delimiter(&bytes[after_end..], BEGIN).is_some() {
        return Err(Error::MultipleBlocks);
    }

    let reference = trimmed_once.trim().to_string();
    if reference.is_empty() {
        return Err(Error::EmptyReference);
    }

    let line_start = text[..begin_pos].rfind('\n').map_or(0, |p| p + 1);
    let line_end = text[after_end..]
        .find('\n')
        .map_or(text.len(), |p| after_end + p + 1);

    Ok(Block {
        reference,
        line_start,
        line_end,
    })
}

/// The result of extracting a manifest block: the reference plus the byte offset
/// and length of the block's line(s) in the source text.
#[derive(Debug)]
pub struct ExtractionResult {
    /// The manifest reference found between the delimiters.
    pub reference: String,
    /// Byte offset of the first byte of the manifest block.
    pub offset: usize,
    /// Byte length of the block, including its line terminator.
    pub length: usize,
}

/// Locate and extract the single manifest block from structured text.
///
/// Returns [`Error::NotFound`] if no block is present, [`Error::MultipleBlocks`]
/// if more than one is present, and [`Error::EmptyReference`] if the reference
/// between the delimiters is empty.
pub fn extract_manifest(text: &str) -> Result<ExtractionResult, Error> {
    let block = locate_block(text)?;
    Ok(ExtractionResult {
        reference: block.reference,
        offset: block.line_start,
        length: block.line_end - block.line_start,
    })
}

/// A classified manifest reference: an external URI or an embedded C2PA Manifest
/// Store decoded from a `data:application/c2pa;base64,` URI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Reference {
    /// A URI to an external C2PA Manifest Store.
    Url(String),
    /// A Manifest Store decoded from an inline `data:` URI.
    Embedded(Vec<u8>),
}

/// Classify and resolve a reference string as extracted from a manifest block.
///
/// A `data:application/c2pa;base64,` URI is decoded to the manifest bytes. Any
/// other value is treated as an external URI and returned verbatim; resolving it
/// over the network is the caller's responsibility (see the `remote` feature).
pub fn classify_reference(reference: &str) -> Result<Reference, Error> {
    if let Some(b64) = reference.strip_prefix(DATA_URI_PREFIX) {
        let bytes = codec::decode(b64).map_err(Error::ManifestDecode)?;
        Ok(Reference::Embedded(bytes))
    } else if reference
        .get(..5)
        .is_some_and(|scheme| scheme.eq_ignore_ascii_case("data:"))
    {
        Err(Error::MalformedReference(reference.to_string()))
    } else if looks_like_uri(reference) {
        Ok(Reference::Url(reference.to_string()))
    } else {
        Err(Error::MalformedReference(reference.to_string()))
    }
}

/// Reject bare CR while accepting LF and CRLF line endings.
pub(crate) fn reject_bare_cr(bytes: &[u8]) -> Result<(), Error> {
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'\r' {
            if bytes.get(i + 1) != Some(&b'\n') {
                return Err(Error::BareCarriageReturn);
            }
            i += 2;
        } else {
            i += 1;
        }
    }
    Ok(())
}

fn looks_like_uri(reference: &str) -> bool {
    // A minimal scheme check: `scheme:` where scheme is ALPHA *( ALPHA / DIGIT
    // / "+" / "-" / "." ) per RFC 3986. Enough to reject stray text without
    // pulling a URI-parsing dependency.
    match reference.find(':') {
        Some(0) | None => false,
        Some(colon) => {
            let scheme = &reference.as_bytes()[..colon];
            scheme[0].is_ascii_alphabetic()
                && scheme
                    .iter()
                    .all(|&c| c.is_ascii_alphanumeric() || matches!(c, b'+' | b'-' | b'.'))
        }
    }
}

/// Locate the first occurrence of `needle` in `haystack` by a byte-window
/// search, returning its start offset.
pub fn find_delimiter(haystack: &[u8], needle: &str) -> Option<usize> {
    let needle = needle.as_bytes();
    haystack.windows(needle.len()).position(|w| w == needle)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_line_python() {
        let text = "# -----BEGIN C2PA MANIFEST----- https://example.com/m.c2pa -----END C2PA MANIFEST-----\nprint('hello')\n";
        let result = extract_manifest(text).unwrap();
        assert_eq!(result.reference, "https://example.com/m.c2pa");
        assert_eq!(result.offset, 0);
    }

    #[test]
    fn front_matter() {
        let text = "---\n-----BEGIN C2PA MANIFEST-----\nhttps://example.com/m.c2pa\n-----END C2PA MANIFEST-----\ntitle: doc\n---\n";
        let result = extract_manifest(text).unwrap();
        assert_eq!(result.reference, "https://example.com/m.c2pa");
        // The excluded span starts after the opening `---` fence, not at 0.
        assert_eq!(result.offset, 4);
    }

    #[test]
    fn not_found() {
        assert!(matches!(
            extract_manifest("no manifest here"),
            Err(Error::NotFound)
        ));
    }

    #[test]
    fn empty_reference() {
        let text = "# -----BEGIN C2PA MANIFEST-----  -----END C2PA MANIFEST-----\n";
        assert!(matches!(extract_manifest(text), Err(Error::EmptyReference)));
    }

    #[test]
    fn multiple_blocks() {
        let text = "# -----BEGIN C2PA MANIFEST----- https://a.com -----END C2PA MANIFEST-----\n# -----BEGIN C2PA MANIFEST----- https://b.com -----END C2PA MANIFEST-----\n";
        assert!(matches!(extract_manifest(text), Err(Error::MultipleBlocks)));
    }

    /// A malformed file with BEGIN and END on non-adjacent lines must not be
    /// treated as a legitimate block -- otherwise everything between them,
    /// including real file content, gets folded into the hash-exclusion range
    /// and can be tampered with undetected.
    #[test]
    fn begin_and_end_on_non_adjacent_lines_is_rejected() {
        let text =
            "# -----BEGIN C2PA MANIFEST----- https://a.com\nprint('x')\n# -----END C2PA MANIFEST-----\nbody\n";
        assert!(matches!(extract_manifest(text), Err(Error::NotFound)));
    }

    /// A second BEGIN before the first block's own END, on the same line as
    /// the reference, is a nested/overlapping block -- not a reference that
    /// happens to mention the word BEGIN.
    #[test]
    fn nested_begin_before_end_is_rejected() {
        let text = "# -----BEGIN C2PA MANIFEST----- https://a.com -----BEGIN C2PA MANIFEST----- https://b.com -----END C2PA MANIFEST-----\n";
        assert!(matches!(extract_manifest(text), Err(Error::MultipleBlocks)));
    }

    #[test]
    fn classify_url() {
        assert_eq!(
            classify_reference("https://example.com/m.c2pa").unwrap(),
            Reference::Url("https://example.com/m.c2pa".to_string())
        );
    }

    #[test]
    fn classify_data_uri() {
        // "foobar" base64-encoded.
        let r = classify_reference("data:application/c2pa;base64,Zm9vYmFy").unwrap();
        assert_eq!(r, Reference::Embedded(b"foobar".to_vec()));
    }

    #[test]
    fn classify_rejects_bare_text() {
        assert!(matches!(
            classify_reference("not a reference"),
            Err(Error::MalformedReference(_))
        ));
    }

    #[test]
    fn classify_rejects_a_data_uri_with_the_wrong_media_type() {
        assert!(matches!(
            classify_reference("data:text/plain;base64,Zm9v"),
            Err(Error::MalformedReference(_))
        ));
    }

    #[test]
    fn extraction_rejects_bare_cr() {
        let text = "# -----BEGIN C2PA MANIFEST----- u -----END C2PA MANIFEST-----\rbody";
        assert!(matches!(
            extract_manifest(text),
            Err(Error::BareCarriageReturn)
        ));
    }
}
