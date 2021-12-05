use std::io::BufRead;

#[cfg(feature = "encoding_rs")]
use encoding_rs::Encoding;

use crate::errors::Xml5Error;
use crate::events::{EmitEvent, Event};
use crate::tokenizer::emitter::{DefaultEmitter, Emitter};

mod decoding;
mod reader;
mod machine;
mod emitter;


pub struct Tokenizer<R: BufRead, E: Emitter = DefaultEmitter> {
    pub(crate) reader: R,
    emitter: E,
    /// which state is the tokenizer in
    state: TokenState,
    /*
        Field related to emitting events
     */
    /// End of file reached - parsing stops
    eof: bool,
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

