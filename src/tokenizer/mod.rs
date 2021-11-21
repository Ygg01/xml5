use std::io::BufRead;
use std::ops::Range;

#[cfg(feature = "encoding_rs")]
use encoding_rs::Encoding;

use crate::errors::{Error};
use crate::events::{Event};

mod decoding;
mod reader;
mod tokenizer;


pub struct Tokenizer<R: BufRead> {
    pub(crate) reader: R,
    /// position of current character
    pos: usize,
    /// which state is the tokenizer in
    state: TokenState,
    event_ready: Event<'static>,
    /*
        Field related to emitting events
     */
    /// Where fragment of text was start and ends
    current_text: Range<usize>,
    /// Where 
    current_tag: Range<usize>,
    /// encoding specified in the xml, or utf8 if none found
    #[cfg(feature = "encoding")]
    encoding: &'static Encoding,
    /// checks if xml5 could identify encoding
    #[cfg(feature = "encoding")]
    is_encoding_set: bool,
}

pub struct TokenResult<'a> {
    pub event: Event<'a>,
    pub error: Error,
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

