use std::str::Utf8Error;

/// A specialized `Result` type where the error is hard-wired to [`Error`].
///
/// [`Error`]: enum.Error.html
pub type TokenizerResult<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    Utf8(Utf8Error),
}

