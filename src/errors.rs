use std::str::Utf8Error;

/// A specialized `Result` type where the error is hard-wired to [`Error`].
///
/// [`Error`]: enum.Error.html
pub type Xml5Result<T> = std::result::Result<T, Xml5Error>;

#[derive(Debug)]
pub enum Xml5Error {
    UnexpectedSymbol(u8),
    Utf8(Utf8Error),
    Io(::std::io::Error),
    NotFound,
    Eof,
}

