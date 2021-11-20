use std::borrow::Cow;

#[derive(Clone, Eq, PartialEq)]
pub struct TagAndAttrText<'a> {
    /// content of the element, before any utf8 conversion
    buf: Cow<'a, [u8]>,
    /// part where name of elements ends
    name_size: usize,
}

#[derive(Clone, Eq, PartialEq)]
pub struct BytesText<'a> {
    /// content of the element, before any utf8 conversion
    buf: Cow<'a, [u8]>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Event<'a> {
    /// Start tag (with attributes) `<tag attr="value">`.
    Start(TagAndAttrText<'a>),
    /// End tag `</tag>`.
    End(TagText<'a>),
    /// Empty element tag (with attributes) `<tag attr="value" />`.
    Empty(TagAndAttrText<'a>),
    /// Character data between `Start` and `End` element.
    Text(BytesText<'a>),
    /// Comment `<!-- ... -->`.
    Comment(BytesText<'a>),
    /// CData `<![CDATA[...]]>`.
    CData(BytesText<'a>),
    /// XML declaration `<?xml ...?>`.
    Decl(BytesText<'a>),
    /// Processing instruction `<?...?>`.
    PI(BytesText<'a>),
    /// Doctype `<!DOCTYPE ...>`.
    DocType(BytesText<'a>),
    /// End of XML document.
    Eof,
}