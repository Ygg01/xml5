use crate::encoding::Decoder;
use crate::errors::Xml5Error;
use std::borrow::Cow;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Token<'a> {
    ///
    Bom(BomText<'a>),
    /// Character data between `Start` and `End` element.
    Text(BytesText<'a>),
    /// Start tag (with attributes) `<tag attr="value">` or `<tag attr="value" />`.
    StartTag(TagAndAttrText<'a>),
    /// End tag `</tag>`, or empty tag `</>`
    EndTag(BytesText<'a>),
    /// Empty tag `<tag attr='x'/>`
    EmtpyTag(TagAndAttrText<'a>),
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

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BomText<'a> {
    pub(crate) buf: Cow<'a, [u8]>,
    /// Encoding in which the `content` is stored inside the event
    pub(crate) decoder: Decoder,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TagAndAttrText<'a> {
    pub(crate) name: Cow<'a, [u8]>,
    pub(crate) self_closing: bool,
    pub(crate) name_len: usize,
}

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct BytesText<'a> {
    pub(crate) name: Cow<'a, [u8]>,
}

impl<'a> BytesText<'a> {
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
