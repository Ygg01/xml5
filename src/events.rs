use crate::encoding::Decoder;
use crate::errors::Xml5Error;
use crate::Token::{EndTag, StartTag};
use std::borrow::Cow;
use std::str::from_utf8_unchecked;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Token<'a> {
    ///
    Bom(EncodedText<'a>),
    /// Character data between `Start` and `End` element.
    Text(BytesText<'a>),
    /// Start tag (with attributes) `<tag attr="value">` or `<tag attr="value" />`.
    StartTag(TagAndAttrText<'a>),
    /// End tag `</tag>`, or empty tag `</>`
    EndTag(BytesText<'a>),
    /// Empty tag `<tag attr='x'/>`
    EmptyTag(TagAndAttrText<'a>),
    /// Comment `<!-- ... -->`.
    Comment(BytesText<'a>),
    /// CData `<![CDATA[...]]>`.
    CData(BytesText<'a>),
    /// XML declaration `<?xml ...?>`.
    Decl(BytesText<'a>),
    /// Processing instruction `<?...?>`.
    PI(PiText<'a>),
    /// Doctype `<!DOCTYPE ...>`.
    DocType(DocTypeText<'a>),
    /// End of XML document.
    Eof,
    /// Error
    Error(Xml5Error),
}

impl<'a> Token<'a> {
    pub fn start_tag(cow: Cow<'a, [u8]>) -> Token<'_> {
        StartTag(TagAndAttrText {
            name: cow,
            self_closing: false,
            name_len: 4,
        })
    }

    pub fn end_tag(cow: Cow<'a, [u8]>) -> Token<'_> {
        EndTag(BytesText { name: cow })
    }
}

impl PartialEq<Token<'_>> for &str {
    fn eq(&self, other: &Token<'_>) -> bool {
        match other {
            Token::Bom(ec) => self.as_bytes() == ec.buf.as_ref(),
            Token::Text(bt)
            | Token::EndTag(bt)
            | Token::CData(bt)
            | Token::Decl(bt)
            | Token::Comment(bt) => self.as_bytes() == bt.name.as_ref(),
            _ => false,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EncodedText<'a> {
    pub(crate) buf: Cow<'a, [u8]>,
    /// Encoding in which the `content` is stored inside the event
    pub(crate) decoder: Decoder,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TagAndAttrText<'a> {
    pub name: Cow<'a, [u8]>,
    pub(crate) self_closing: bool,
    pub(crate) name_len: usize,
}

impl<'a> TagAndAttrText<'a> {
    #[cfg(feature = "encoding")]
    pub fn name_as_str(&self, decoding: Decoder) -> crate::encoding::Result<Cow<'_, str>> {
        unsafe {
            match &self.name {
                Cow::Borrowed(x) => decoding.decode(x),
                Cow::Owned(x) => decoding.decode(x),
            }
        }
    }

    #[cfg(not(feature = "encoding"))]
    pub fn name_as_str(&self, decoding: Decoder) -> &str {
        unsafe {
            match &self.name {
                Cow::Borrowed(x) => from_utf8_unchecked(x),
                Cow::Owned(x) => from_utf8_unchecked(x),
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct BytesText<'a> {
    pub(crate) name: Cow<'a, [u8]>,
}

impl<'a> BytesText<'a> {
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.name.is_empty()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PiText<'a> {
    pub(crate) name: Cow<'a, [u8]>,
    pub(crate) value: Cow<'a, [u8]>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DocTypeText<'a> {
    pub(crate) name: Cow<'a, [u8]>,
}
