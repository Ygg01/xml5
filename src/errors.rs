use std::str::Utf8Error;

/// A specialized `Result` type where the error is hard-wired to [`Error`].
///
/// [`Error`]: enum.Error.html
pub type Xml5Result<T> = std::result::Result<T, Xml5Error>;

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Xml5Error {
    UnexpectedSymbol(Option<u8>),
    Utf8(Utf8Error),
    Io(String),
    NotFound,
    Eof,
}

