const BEGIN: &str = "-----BEGIN C2PA MANIFEST-----";
const END: &str = "-----END C2PA MANIFEST-----";
const B64_CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

pub enum ManifestRef<'a> {
    Url(&'a str),
    Embedded(&'a [u8]),
}

pub fn embed_manifest(
    text: &str,
    manifest: ManifestRef<'_>,
    comment_prefix: &str,
    comment_suffix: Option<&str>,
) -> String {
    let reference = match manifest {
        ManifestRef::Url(url) => url.to_string(),
        ManifestRef::Embedded(bytes) => {
            format!("data:application/c2pa;base64,{}", base64_encode(bytes))
        }
    };

    let suffix = comment_suffix.unwrap_or("");
    let manifest_line = format!(
        "{comment_prefix} {BEGIN} {reference} {END} {suffix}"
    ).trim_end().to_string();

    format!("{manifest_line}\n{text}")
}

fn base64_encode(input: &[u8]) -> String {
    let mut out = Vec::with_capacity((input.len() + 2) / 3 * 4);
    for chunk in input.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = if chunk.len() > 1 { chunk[1] as u32 } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] as u32 } else { 0 };
        let triple = (b0 << 16) | (b1 << 8) | b2;
        out.push(B64_CHARS[((triple >> 18) & 0x3F) as usize]);
        out.push(B64_CHARS[((triple >> 12) & 0x3F) as usize]);
        if chunk.len() > 1 {
            out.push(B64_CHARS[((triple >> 6) & 0x3F) as usize]);
        } else {
            out.push(b'=');
        }
        if chunk.len() > 2 {
            out.push(B64_CHARS[(triple & 0x3F) as usize]);
        } else {
            out.push(b'=');
        }
    }
    String::from_utf8(out).unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embed_url_python() {
        let text = "print('hello')\n";
        let result = embed_manifest(text, ManifestRef::Url("https://example.com/m.c2pa"), "#", None);
        assert!(result.starts_with("# -----BEGIN C2PA MANIFEST-----"));
        assert!(result.contains("https://example.com/m.c2pa"));
        assert!(result.contains("-----END C2PA MANIFEST-----"));
        assert!(result.ends_with("print('hello')\n"));
    }

    #[test]
    fn embed_url_css() {
        let result = embed_manifest("body {}", ManifestRef::Url("https://example.com/m.c2pa"), "/*", Some("*/"));
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
    fn base64_known_vectors() {
        assert_eq!(base64_encode(b""), "");
        assert_eq!(base64_encode(b"f"), "Zg==");
        assert_eq!(base64_encode(b"fo"), "Zm8=");
        assert_eq!(base64_encode(b"foo"), "Zm9v");
        assert_eq!(base64_encode(b"foob"), "Zm9vYg==");
        assert_eq!(base64_encode(b"fooba"), "Zm9vYmE=");
        assert_eq!(base64_encode(b"foobar"), "Zm9vYmFy");
    }
}
