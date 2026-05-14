use std::fmt;

#[derive(Debug)]
pub enum Error {
    NotFound,
    MultipleBlocks,
    EmptyReference,
    MalformedReference(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound => write!(f, "no manifest block found"),
            Self::MultipleBlocks => write!(f, "multiple manifest blocks found"),
            Self::EmptyReference => write!(f, "empty manifest reference"),
            Self::MalformedReference(s) => write!(f, "malformed manifest reference: {s}"),
        }
    }
}

impl std::error::Error for Error {}
