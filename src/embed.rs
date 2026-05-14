use base64::{Engine, engine::general_purpose::STANDARD};

const BEGIN: &str = "-----BEGIN C2PA MANIFEST-----";
const END: &str = "-----END C2PA MANIFEST-----";

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
            format!("data:application/c2pa;base64,{}", STANDARD.encode(bytes))
        }
    };

    let suffix = comment_suffix.unwrap_or("");
    let manifest_line = format!(
        "{comment_prefix} {BEGIN} {reference} {END} {suffix}"
    ).trim_end().to_string();

    format!("{manifest_line}\n{text}")
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
}
