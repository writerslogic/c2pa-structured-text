// Copyright 2026 WritersLogic. All rights reserved.
// Licensed under the Apache License, Version 2.0 or the MIT license,
// at your option.

//! Standard (RFC 4648) Base64 encode/decode with no external dependencies.
//!
//! Used for two purposes:
//! - Encoding an embedded C2PA Manifest Store into a `data:application/c2pa;base64,`
//!   reference and decoding it back out again ([`crate::extract`]).
//! - Encoding the hard-binding hash value when serialising a `c2pa.hash.data`
//!   assertion to JSON ([`crate::hardbinding`]).

const CHARS: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

pub fn encode(input: &[u8]) -> String {
    let mut out = Vec::with_capacity(input.len().div_ceil(3) * 4);
    for chunk in input.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = if chunk.len() > 1 { chunk[1] as u32 } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] as u32 } else { 0 };
        let triple = (b0 << 16) | (b1 << 8) | b2;
        out.push(CHARS[((triple >> 18) & 0x3F) as usize]);
        out.push(CHARS[((triple >> 12) & 0x3F) as usize]);
        if chunk.len() > 1 {
            out.push(CHARS[((triple >> 6) & 0x3F) as usize]);
        } else {
            out.push(b'=');
        }
        if chunk.len() > 2 {
            out.push(CHARS[(triple & 0x3F) as usize]);
        } else {
            out.push(b'=');
        }
    }
    // Every byte pushed is drawn from `CHARS` or is `b'='`, so the result is ASCII.
    String::from_utf8(out).expect("base64 alphabet is valid UTF-8")
}

/// Decode standard Base64. Rejects any character outside the alphabet (including
/// URL-safe `-`/`_`) and any malformed padding. Whitespace is not permitted;
/// callers must trim before decoding.
pub fn decode(input: &str) -> Result<Vec<u8>, DecodeError> {
    let bytes = input.as_bytes();
    if !bytes.len().is_multiple_of(4) {
        return Err(DecodeError::InvalidLength);
    }
    if bytes.is_empty() {
        return Ok(Vec::new());
    }

    let mut out = Vec::with_capacity(bytes.len() / 4 * 3);
    for (block_idx, block) in bytes.chunks(4).enumerate() {
        let is_last = block_idx == bytes.len() / 4 - 1;
        let mut acc = 0u32;
        let mut pad = 0usize;
        for (i, &c) in block.iter().enumerate() {
            let value = match c {
                b'A'..=b'Z' => c - b'A',
                b'a'..=b'z' => c - b'a' + 26,
                b'0'..=b'9' => c - b'0' + 52,
                b'+' => 62,
                b'/' => 63,
                b'=' => {
                    // Padding is only legal in the final block, and only in the
                    // last two positions.
                    if !is_last || i < 2 {
                        return Err(DecodeError::InvalidPadding);
                    }
                    pad += 1;
                    0
                }
                _ => return Err(DecodeError::InvalidCharacter(c)),
            };
            // A non-pad character after a pad character is malformed (e.g. "AB=C").
            if c != b'=' && pad != 0 {
                return Err(DecodeError::InvalidPadding);
            }
            acc = (acc << 6) | value as u32;
        }
        out.push((acc >> 16) as u8);
        if pad < 2 {
            out.push((acc >> 8) as u8);
        }
        if pad < 1 {
            out.push(acc as u8);
        }
    }
    Ok(out)
}

#[derive(Debug, PartialEq, Eq)]
#[allow(clippy::enum_variant_names)]
pub enum DecodeError {
    InvalidLength,
    InvalidPadding,
    InvalidCharacter(u8),
}

impl core::fmt::Display for DecodeError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::InvalidLength => write!(f, "base64 input length is not a multiple of 4"),
            Self::InvalidPadding => write!(f, "base64 input has malformed padding"),
            Self::InvalidCharacter(c) => write!(f, "base64 input has invalid character {c:#04x}"),
        }
    }
}

impl std::error::Error for DecodeError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_known_vectors() {
        assert_eq!(encode(b""), "");
        assert_eq!(encode(b"f"), "Zg==");
        assert_eq!(encode(b"fo"), "Zm8=");
        assert_eq!(encode(b"foo"), "Zm9v");
        assert_eq!(encode(b"foob"), "Zm9vYg==");
        assert_eq!(encode(b"fooba"), "Zm9vYmE=");
        assert_eq!(encode(b"foobar"), "Zm9vYmFy");
    }

    #[test]
    fn decode_known_vectors() {
        assert_eq!(decode("").unwrap(), b"");
        assert_eq!(decode("Zg==").unwrap(), b"f");
        assert_eq!(decode("Zm8=").unwrap(), b"fo");
        assert_eq!(decode("Zm9v").unwrap(), b"foo");
        assert_eq!(decode("Zm9vYg==").unwrap(), b"foob");
        assert_eq!(decode("Zm9vYmE=").unwrap(), b"fooba");
        assert_eq!(decode("Zm9vYmFy").unwrap(), b"foobar");
    }

    #[test]
    fn round_trip_binary() {
        let data: Vec<u8> = (0u16..=255).map(|b| b as u8).collect();
        assert_eq!(decode(&encode(&data)).unwrap(), data);
    }

    #[test]
    fn decode_rejects_bad_input() {
        assert_eq!(decode("Zg="), Err(DecodeError::InvalidLength));
        assert_eq!(decode("Zg=v"), Err(DecodeError::InvalidPadding));
        assert_eq!(decode("Z-9v"), Err(DecodeError::InvalidCharacter(b'-')));
        assert_eq!(decode("====").unwrap_err(), DecodeError::InvalidPadding);
    }
}
