use crate::errors::Xml5Error;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Token {
    /// Character data between `Start` and `End` element.
    Text(String),
    /// Start tag (with attributes) `<tag attr="value">`.
    StartTag(TagAndAttrText),
    /// End tag `</tag>`.
    EndTag(String),
    /// Empty element tag (with attributes) `<tag attr="value" />`.
    EmptyTag(TagAndAttrText),
    /// Comment `<!-- ... -->`.
    Comment(String),
    /// CData `<![CDATA[...]]>`.
    CData(String),
    /// XML declaration `<?xml ...?>`.
    Decl(String),
    /// Processing instruction `<?...?>`.
    PI(String),
    /// Doctype `<!DOCTYPE ...>`.
    DocType(String),
    /// End of XML document.
    Eof,
    /// Error
    Error(Xml5Error),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TagAndAttrText {

}

