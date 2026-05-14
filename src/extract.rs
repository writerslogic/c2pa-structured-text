use crate::error::Error;

const BEGIN: &str = "-----BEGIN C2PA MANIFEST-----";
const END: &str = "-----END C2PA MANIFEST-----";

#[derive(Debug)]
pub struct ExtractionResult {
    pub reference: String,
    pub offset: usize,
    pub length: usize,
}

pub fn extract_manifest(text: &str) -> Result<ExtractionResult, Error> {
    let bytes = text.as_bytes();

    let begin_pos = match find_delimiter(bytes, BEGIN) {
        Some(pos) => pos,
        None => return Err(Error::NotFound),
    };

    let after_begin = begin_pos + BEGIN.len();

    let end_pos = match find_delimiter(&bytes[after_begin..], END) {
        Some(pos) => after_begin + pos,
        None => return Err(Error::NotFound),
    };

    if find_delimiter(&bytes[end_pos + END.len()..], BEGIN).is_some() {
        return Err(Error::MultipleBlocks);
    }

    let reference = text[after_begin..end_pos].trim().to_string();

    if reference.is_empty() {
        return Err(Error::EmptyReference);
    }

    let line_start = text[..begin_pos].rfind('\n').map_or(0, |p| p + 1);
    let line_end = text[end_pos + END.len()..]
        .find('\n')
        .map_or(text.len(), |p| end_pos + END.len() + p + 1);

    Ok(ExtractionResult {
        reference,
        offset: line_start,
        length: line_end - line_start,
    })
}

fn find_delimiter(haystack: &[u8], needle: &str) -> Option<usize> {
    let needle = needle.as_bytes();
    haystack
        .windows(needle.len())
        .position(|w| w == needle)
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
        assert!(matches!(
            extract_manifest(text),
            Err(Error::EmptyReference)
        ));
    }

    #[test]
    fn multiple_blocks() {
        let text = "# -----BEGIN C2PA MANIFEST----- https://a.com -----END C2PA MANIFEST-----\n# -----BEGIN C2PA MANIFEST----- https://b.com -----END C2PA MANIFEST-----\n";
        assert!(matches!(
            extract_manifest(text),
            Err(Error::MultipleBlocks)
        ));
    }
}
