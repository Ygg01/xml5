use std::str::Utf8Error;

/// A specialized `Result` type where the error is hard-wired to [`Error`].
///
/// [`Error`]: enum.Error.html
pub type Xml5Result<T> = std::result::Result<T, Xml5Error>;

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Xml5Error {
    AbruptClosingEmptyComment,
    AbruptClosingXmlDeclaration,
    AbruptEndDoctypeIdentifier,
    ColonBeforeAttrName,
    EofInCdata,
    EofInComment,
    EofInDoctype,
    EofInTag,
    EofInXmlDeclaration,
    GreaterThanInComment,
    InvalidCharactersInAfterDoctypeName,
    IncorrectlyOpenedComment,
    MissingWhitespaceDoctype,
    MissingWhitespaceAfterDoctypeKeyword,
    MissingWhitespaceBetweenDoctypePublicAndSystem,
    MissingQuoteBeforeIdentifier,
    MissingDoctypeName,
    MissingDoctypeIdentifier,
    InvalidXmlDeclaration,
    UnexpectedSymbolOrEof(Option<u8>),
    UnexpectedSymbol(char),
    UnexpectedEof,
    Utf8(Utf8Error),
    Io(String),
    /// Input decoding error. If `encoding` feature is disabled, contains `None`,
    /// otherwise contains the UTF-8 decoding error
    NonDecodable(Option<Utf8Error>),
    NotFound,
}

impl From<::std::io::Error> for Xml5Error {
    /// Creates a new `Error::Io` from the given error
    #[inline]
    fn from(error: ::std::io::Error) -> Xml5Error {
        Xml5Error::Io(error.to_string())
    }
}

impl From<Utf8Error> for Xml5Error {
    /// Creates a new `Error::NonDecodable` from the given error
    #[inline]
    fn from(error: Utf8Error) -> Xml5Error {
        Xml5Error::NonDecodable(Some(error))
    }
}
