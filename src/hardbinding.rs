// Copyright 2026 WritersLogic. All rights reserved.
// Licensed under the Apache License, Version 2.0 or the MIT license,
// at your option.

//! The hard binding for structured text.
//!
//! # What the binding covers
//!
//! A claim generator that embeds a manifest block in a structured text file
//! includes a `c2pa.hash.data` assertion with a **single exclusion range
//! covering the entire manifest block**. The hash is computed over the raw
//! bytes of the file with that range removed.
//!
//! Unlike the Unicode Variation Selector method for *unstructured* text, this
//! binding applies **no Unicode normalization**. Structured text files are
//! byte-stable on disk; normalizing to NFC would create false mismatches for
//! files that legitimately contain NFD content (identifiers, YAML values,
//! filenames). The byte boundaries of the manifest block are fixed by the ASCII
//! delimiters, so the exclusion range is unambiguous. Files must therefore be
//! read in binary mode, preserving exact line terminators. Bare CR (0x0D) line
//! endings are unsupported and rejected.
//!
//! # Fragility and the soft-binding recovery path
//!
//! This is a *byte-exact* binding. Any change to the covered bytes -- including
//! reformatting, re-indentation, transcoding, or an LF/CRLF conversion outside
//! the manifest block -- breaks it, exactly as a hard binding is meant to. When
//! durability across such transformations is required, pair it with the
//! perceptual soft binding in `c2pa-text-binding`, which re-associates
//! transformed content with its provenance after the hard binding is lost. Do
//! not treat the structured-text hard binding as robust to editing; it is not.

use crate::error::Error;
use crate::extract::locate_block;

/// A byte range excluded from the data hash, matching the `EXCLUSION_RANGE-map`
/// CDDL (`start`, `length`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Exclusion {
    pub start: usize,
    pub length: usize,
}

/// The assertion label for the structured-text hard binding.
pub const DATA_HASH_LABEL: &str = "c2pa.hash.data";

/// Compute the single exclusion range covering the manifest block, per the
/// placement rules in the C2PA specification (§ *Hard Binding* for structured
/// text).
///
/// Placement is derived from where the block sits in the file:
/// - beginning: `start = 0`, `length` = block through its trailing terminator;
/// - end: `start` = the newline preceding the block, `length` to end of file;
/// - elsewhere (e.g. after a reserved header or inside front matter):
///   `start` = first byte of the block line, `length` = the block line and its
///   trailing terminator;
/// - whole file is the block: `start = 0`, `length` = file length.
pub fn manifest_exclusion(text: &str) -> Result<Exclusion, Error> {
    reject_bare_cr(text.as_bytes())?;
    let block = locate_block(text)?;
    let len = text.len();
    let bs = block.line_start;
    let be = block.line_end;

    let exclusion = if bs == 0 && be == len {
        Exclusion {
            start: 0,
            length: len,
        }
    } else if bs == 0 {
        Exclusion {
            start: 0,
            length: be,
        }
    } else if be == len {
        // End placement: also exclude the line terminator that precedes the
        // block so that removing the range leaves no dangling newline.
        let bytes = text.as_bytes();
        let mut start = bs - 1; // the LF; guaranteed present because bs > 0.
        if start > 0 && bytes[start - 1] == b'\r' {
            start -= 1;
        }
        Exclusion {
            start,
            length: len - start,
        }
    } else {
        Exclusion {
            start: bs,
            length: be - bs,
        }
    };

    Ok(exclusion)
}

/// Return the exact byte sequence that the data hash covers: the file with the
/// manifest block excluded. This is the seam shared by hash computation and by
/// the `c2pa-rs` validation bridge.
pub fn hashed_bytes(text: &str) -> Result<Vec<u8>, Error> {
    let exclusion = manifest_exclusion(text)?;
    apply_exclusions(text.as_bytes(), &[exclusion])
}

/// Remove `exclusions` from `bytes`, validating that they are ordered,
/// non-overlapping, and within bounds as a validator must (see *Validating a
/// data hash*).
pub(crate) fn apply_exclusions(bytes: &[u8], exclusions: &[Exclusion]) -> Result<Vec<u8>, Error> {
    let mut cursor = 0usize;
    let mut out = Vec::with_capacity(bytes.len());
    for ex in exclusions {
        let end = ex
            .start
            .checked_add(ex.length)
            .ok_or(Error::MalformedExclusion)?;
        if ex.start < cursor {
            // Out of order or overlapping with a previous range.
            return Err(Error::MalformedExclusion);
        }
        if end > bytes.len() {
            // End of an exclusion range beyond the end of the asset.
            return Err(Error::HashMismatch);
        }
        out.extend_from_slice(&bytes[cursor..ex.start]);
        cursor = end;
    }
    out.extend_from_slice(&bytes[cursor..]);
    Ok(out)
}

fn reject_bare_cr(bytes: &[u8]) -> Result<(), Error> {
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

#[cfg(feature = "hard-binding")]
mod hashing {
    use super::{apply_exclusions, manifest_exclusion, reject_bare_cr, Exclusion, DATA_HASH_LABEL};
    use crate::codec;
    use crate::error::Error;
    use sha2::{Digest, Sha256, Sha384, Sha512};

    /// A C2PA-allowed hash algorithm for the data hash (SHA2-256/384/512).
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum Algorithm {
        Sha256,
        Sha384,
        Sha512,
    }

    impl Algorithm {
        /// The C2PA algorithm identifier string used in the `alg` field.
        pub fn id(self) -> &'static str {
            match self {
                Algorithm::Sha256 => "sha256",
                Algorithm::Sha384 => "sha384",
                Algorithm::Sha512 => "sha512",
            }
        }

        pub fn from_id(id: &str) -> Result<Self, Error> {
            match id {
                "sha256" => Ok(Algorithm::Sha256),
                "sha384" => Ok(Algorithm::Sha384),
                "sha512" => Ok(Algorithm::Sha512),
                other => Err(Error::UnsupportedAlgorithm(other.to_string())),
            }
        }

        fn hash(self, data: &[u8]) -> Vec<u8> {
            match self {
                Algorithm::Sha256 => Sha256::digest(data).to_vec(),
                Algorithm::Sha384 => Sha384::digest(data).to_vec(),
                Algorithm::Sha512 => Sha512::digest(data).to_vec(),
            }
        }
    }

    /// A computed `c2pa.hash.data` assertion for a structured text asset.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct DataHash {
        pub exclusions: Vec<Exclusion>,
        pub alg: String,
        pub hash: Vec<u8>,
        pub name: Option<String>,
    }

    impl DataHash {
        /// The assertion label, `c2pa.hash.data`.
        pub fn label(&self) -> &'static str {
            DATA_HASH_LABEL
        }

        /// Serialise to the JSON shape consumed by `c2pa-rs` when building a
        /// manifest, with the hash as standard Base64. Hand-built to keep the
        /// crate dependency-light; the field set matches the `data-hash-map`
        /// CDDL.
        pub fn to_json(&self) -> String {
            let ranges: Vec<String> = self
                .exclusions
                .iter()
                .map(|e| format!("{{\"start\":{},\"length\":{}}}", e.start, e.length))
                .collect();
            let mut json = format!(
                "{{\"exclusions\":[{}],\"alg\":\"{}\",\"hash\":\"{}\"",
                ranges.join(","),
                self.alg,
                codec::encode(&self.hash)
            );
            if let Some(name) = &self.name {
                json.push_str(&format!(",\"name\":\"{}\"", name));
            }
            json.push('}');
            json
        }
    }

    /// Compute the structured-text hard binding for `text`: locate the manifest
    /// block, exclude it, and hash the remaining raw bytes with `alg`.
    pub fn compute_data_hash(text: &str, alg: Algorithm) -> Result<DataHash, Error> {
        let exclusion = manifest_exclusion(text)?;
        let covered = apply_exclusions(text.as_bytes(), &[exclusion])?;
        Ok(DataHash {
            exclusions: vec![exclusion],
            alg: alg.id().to_string(),
            hash: alg.hash(&covered),
            name: None,
        })
    }

    /// Verify a `c2pa.hash.data` binding against `text`, following the validator
    /// procedure: apply the assertion's own exclusion ranges to the raw bytes,
    /// recompute the hash, and compare.
    ///
    /// Returns [`Error::HashMismatch`] on a content mismatch,
    /// [`Error::MalformedExclusion`] on out-of-order or overlapping ranges, and
    /// [`Error::UnsupportedAlgorithm`] if `alg` is outside the allowed list.
    pub fn verify_data_hash(text: &str, data_hash: &DataHash) -> Result<(), Error> {
        reject_bare_cr(text.as_bytes())?;
        let alg = Algorithm::from_id(&data_hash.alg)?;
        if data_hash.exclusions.is_empty() {
            return Err(Error::MalformedExclusion);
        }
        let covered = apply_exclusions(text.as_bytes(), &data_hash.exclusions)?;
        if alg.hash(&covered) == data_hash.hash {
            Ok(())
        } else {
            Err(Error::HashMismatch)
        }
    }
}

#[cfg(feature = "hard-binding")]
pub use hashing::{compute_data_hash, verify_data_hash, Algorithm, DataHash};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::embed::{embed_front_matter, embed_manifest, embed_manifest_at_end, ManifestRef};

    const URL: &str = "https://example.com/m.c2pa";

    #[test]
    fn exclusion_at_beginning() {
        let embedded = embed_manifest("print('hi')\n", ManifestRef::Url(URL), "#", None);
        let ex = manifest_exclusion(&embedded).unwrap();
        assert_eq!(ex.start, 0);
        // Removing the exclusion recovers the original content exactly.
        assert_eq!(hashed_bytes(&embedded).unwrap(), b"print('hi')\n");
    }

    #[test]
    fn exclusion_at_end_excludes_preceding_newline() {
        let embedded = embed_manifest_at_end("print('hi')\n", ManifestRef::Url(URL), "#", None);
        let ex = manifest_exclusion(&embedded).unwrap();
        // The block sits at EOF; the newline before it is part of the range.
        assert_eq!(ex.start + ex.length, embedded.len());
        assert_eq!(hashed_bytes(&embedded).unwrap(), b"print('hi')");
    }

    #[test]
    fn exclusion_elsewhere_after_header() {
        // A reserved first line (shebang) with the block on line two, content after.
        let text =
            "#!/bin/sh\n# -----BEGIN C2PA MANIFEST----- u -----END C2PA MANIFEST-----\necho hi\n";
        let ex = manifest_exclusion(text).unwrap();
        assert_eq!(ex.start, "#!/bin/sh\n".len());
        assert_eq!(hashed_bytes(text).unwrap(), b"#!/bin/sh\necho hi\n");
    }

    #[test]
    fn exclusion_front_matter_keeps_fences() {
        let embedded = embed_front_matter("title: doc\n", ManifestRef::Url(URL), "---");
        // The `---` fences are preserved; only BEGIN..END lines are excluded.
        let covered = String::from_utf8(hashed_bytes(&embedded).unwrap()).unwrap();
        assert!(covered.starts_with("---\n"));
        assert!(covered.contains("title: doc"));
        assert!(!covered.contains("BEGIN C2PA MANIFEST"));
    }

    #[test]
    fn exclusion_whole_file_is_block() {
        let text = "# -----BEGIN C2PA MANIFEST----- u -----END C2PA MANIFEST-----\n";
        let ex = manifest_exclusion(text).unwrap();
        assert_eq!(ex.start, 0);
        assert_eq!(ex.length, text.len());
        assert_eq!(hashed_bytes(text).unwrap(), b"");
    }

    #[test]
    fn crlf_is_supported() {
        let text = "# -----BEGIN C2PA MANIFEST----- u -----END C2PA MANIFEST-----\r\nprint()\r\n";
        assert_eq!(hashed_bytes(text).unwrap(), b"print()\r\n");
    }

    #[test]
    fn bare_cr_is_rejected() {
        let text = "# -----BEGIN C2PA MANIFEST----- u -----END C2PA MANIFEST-----\rprint()";
        assert!(matches!(
            manifest_exclusion(text),
            Err(Error::BareCarriageReturn)
        ));
    }

    #[test]
    fn apply_exclusions_rejects_out_of_order() {
        let bytes = b"0123456789";
        let ranges = [
            Exclusion {
                start: 5,
                length: 2,
            },
            Exclusion {
                start: 1,
                length: 2,
            },
        ];
        assert!(matches!(
            apply_exclusions(bytes, &ranges),
            Err(Error::MalformedExclusion)
        ));
    }

    #[test]
    fn apply_exclusions_rejects_beyond_end() {
        let bytes = b"0123456789";
        let ranges = [Exclusion {
            start: 8,
            length: 5,
        }];
        assert!(matches!(
            apply_exclusions(bytes, &ranges),
            Err(Error::HashMismatch)
        ));
    }
}
