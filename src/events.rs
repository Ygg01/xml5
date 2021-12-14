use crate::errors::Xml5Error;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Token {
    /// Character data between `Start` and `End` element.
    Text(Vec<u8>),
    /// Start tag (with attributes) `<tag attr="value">`.
    StartTag(TagAndAttrText),
    /// End tag `</tag>`.
    EndTag(Vec<u8>),
    /// Short tag `</>`
    ShortTag,
    /// Empty element tag (with attributes) `<tag attr="value" />`.
    EmptyTag(TagAndAttrText),
    /// Comment `<!-- ... -->`.
    Comment(Vec<u8>),
    /// CData `<![CDATA[...]]>`.
    CData(Vec<u8>),
    /// XML declaration `<?xml ...?>`.
    Decl(Vec<u8>),
    /// Processing instruction `<?...?>`.
    PI(Vec<u8>),
    /// Doctype `<!DOCTYPE ...>`.
    DocType(Vec<u8>),
    /// End of XML document.
    Eof,
    /// Error
    Error(Xml5Error),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TagAndAttrText {

}

