#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("no manifest block found")]
    NotFound,

    #[error("multiple manifest blocks found")]
    MultipleBlocks,

    #[error("empty manifest reference")]
    EmptyReference,

    #[error("malformed manifest reference: {0}")]
    MalformedReference(String),

    #[error("invalid base64: {0}")]
    Base64(#[from] base64::DecodeError),
}
