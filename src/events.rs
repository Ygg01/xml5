use std::borrow::Cow;
use std::borrow::Cow::Borrowed;
use std::ops::Deref;
use std::str::from_utf8_unchecked;

use crate::encoding::Decoder;
use crate::errors::Xml5Error;

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
    Declaration(BytesText<'a>),
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
    #[inline]
    pub fn start_tag(cow: Cow<'a, [u8]>, attrs: Vec<(Cow<'a, [u8]>, Cow<'a, [u8]>)>) -> Token<'a> {
        Token::StartTag(TagAndAttrText {
            name: cow,
            attrs,
            self_closing: false,
        })
    }

    #[inline]
    pub fn empty_tag(cow: Cow<'a, [u8]>, attrs: Vec<(Cow<'a, [u8]>, Cow<'a, [u8]>)>) -> Token<'a> {
        Token::StartTag(TagAndAttrText {
            name: cow,
            attrs,
            self_closing: true,
        })
    }

    #[inline]
    pub fn end_tag(name: Cow<'a, [u8]>) -> Token<'_> {
        Token::EndTag(BytesText { name })
    }

    #[inline]
    pub fn pi_tag(data: Cow<'a, [u8]>, target: Cow<'a, [u8]>) -> Token<'a> {
        Token::PI(PiText { data, target })
    }

    #[inline]
    pub fn comment(name: Cow<'a, [u8]>) -> Token<'a> {
        Token::Comment(BytesText { name })
    }

    #[inline]
    pub fn declaration(text: Cow<'a, [u8]>) -> Token<'a> {
        Token::Declaration(BytesText { name: text })
    }

    #[inline]
    pub fn cdata(text: Cow<'a, [u8]>) -> Token<'a> {
        Token::CData(BytesText { name: text })
    }

    #[inline]
    pub fn auto_close_tag() -> Token<'a> {
        Token::EndTag(BytesText {
            name: Cow::default(),
        })
    }

    #[inline]
    pub fn text(text: Cow<'a, [u8]>) -> Token<'a> {
        Token::Text(BytesText { name: text })
    }

    #[inline]
    pub fn error(err: Xml5Error) -> Token<'a> {
        Token::Error(err)
    }

    #[inline]
    pub fn get_name(&self) -> Option<&[u8]> {
        match self {
            Token::StartTag(start) => Some(start.name.deref()),
            Token::EndTag(end) => Some(end.name.deref()),
            _ => None,
        }
    }

    #[inline]
    pub fn get_target(&self) -> Option<&[u8]> {
        match self {
            Token::PI(pi) => Some(pi.get_target()),
            _ => None,
        }
    }

    #[inline]
    pub fn get_data(&self) -> Option<&[u8]> {
        match self {
            Token::PI(pi) => Some(pi.get_data()),
            _ => None,
        }
    }

    #[inline]
    pub fn get_text(&self) -> Option<&[u8]> {
        match self {
            Token::Text(text) | Token::CData(text) => Some(text.as_ref()),
            _ => None,
        }
    }

    #[inline]
    pub fn get_declaration(&self) -> Option<&[u8]> {
        match self {
            Token::Declaration(text) => Some(text.as_ref()),
            _ => None,
        }
    }
}

impl PartialEq<BytesText<'_>> for &str {
    fn eq(&self, other: &BytesText<'_>) -> bool {
        self.as_bytes() == other.name.as_ref()
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
    pub attrs: Vec<(Cow<'a, [u8]>, Cow<'a, [u8]>)>,
    pub(crate) self_closing: bool,
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

impl<'a> Deref for TagAndAttrText<'a> {
    type Target = [u8];

    fn deref(&self) -> &[u8] {
        self.name.deref()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct BytesText<'a> {
    pub(crate) name: Cow<'a, [u8]>,
}

impl<'a> Deref for BytesText<'a> {
    type Target = [u8];

    fn deref(&self) -> &[u8] {
        self.name.deref()
    }
}

impl<'a> BytesText<'a> {
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.name.is_empty()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PiText<'a> {
    pub(crate) target: Cow<'a, [u8]>,
    pub(crate) data: Cow<'a, [u8]>,
}

impl<'a> PiText<'a> {
    pub fn get_target(&self) -> &[u8] {
        self.target.deref()
    }

    pub fn get_data(&self) -> &[u8] {
        self.data.deref()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DocTypeText<'a> {
    pub(crate) name: Cow<'a, [u8]>,
}
