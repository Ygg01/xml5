use std::io::BufRead;

#[cfg(feature = "encoding_rs")]
use encoding_rs::Encoding;

use crate::errors::Xml5Error;
use crate::events::{EmitEvent, Event};

mod decoding;
mod reader;
mod machine;


pub struct Tokenizer<R: BufRead> {
    pub(crate) reader: R,
    /// position of current character
    pos: usize,
    /// which state is the tokenizer in
    state: TokenState,
    /*
        Field related to emitting events
     */
    ///
    events_to_emit: Vec<EmitEvent>,
    /// Where fragment of text was start and ends
    current_text: Vec<u8>,
    /// Where
    current_tag: Vec<u8>,
    /// encoding specified in the xml, or utf8 if none found
    #[cfg(feature = "encoding")]
    encoding: &'static Encoding,
    /// checks if xml5 could identify encoding
    #[cfg(feature = "encoding")]
    is_encoding_set: bool,
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum RangeOrChar {
    SliceRange(usize, usize),
    Char(char),
}

#[derive(Debug, Clone, Copy)]
#[doc(hidden)]
enum TokenState {
    Data,
    CharRefInData,
    Tag,
    EndTag,
    EndTagName,
    EndTagNameAfter,
    Pi,
    PiTarget,
    PiTargetAfter,
    PiData,
    PiAfter,
    MarkupDecl,
    CommentStart,
    CommentStartDash,
    Comment,
    CommentLessThan,
    CommentLessThanBang,
    CommentLessThanBangDash,
    CommentLessThanBangDashDash,
    CommentEnd,
    CommentEndDash,
    CommentEndBang,
    Cdata,
    CdataBracket,
    CdataEnd,
    TagName,
    TagEmpty,
    TagAttrNameBefore,
    TagAttrName,
    TagAttrNameAfter,
    TagAttrValueBefore,
    TagAttrValue(AttrValueKind),
    Doctype,
    BeforeDoctypeName,
    DoctypeName,
    AfterDoctypeName,
    AfterDoctypeKeyword(DoctypeKind),
    BeforeDoctypeIdentifier(DoctypeKind),
    DoctypeIdentifierDoubleQuoted(DoctypeKind),
    DoctypeIdentifierSingleQuoted(DoctypeKind),
    AfterDoctypeIdentifier(DoctypeKind),
    BetweenDoctypePublicAndSystemIdentifiers,
    BogusDoctype,
    BogusComment,
    Quiescent,
}

#[derive(Debug, Clone, Copy)]
#[doc(hidden)]
pub enum AttrValueKind {
    Unquoted,
    SingleQuoted,
    DoubleQuoted,
}

#[derive(Debug, Clone, Copy)]
#[doc(hidden)]
pub enum DoctypeKind {
    Public,
    System,
}

