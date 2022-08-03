use std::io::{BufRead};

#[cfg(feature = "encoding_rs")]
use encoding_rs::Encoding;
use crate::errors::{Xml5Error, Xml5Result};
use crate::Token;

use crate::tokenizer::emitter::{DefaultEmitter, Emitter};

mod decoding;
mod reader;
mod machine;
mod emitter;


pub struct Tokenizer<'b, S: BufRead, E: Emitter = DefaultEmitter> {
    pub(crate) source: S,
    pub(crate) buffer: &'b mut Vec<u8>,
    emitter: E,
    /// which state is the tokenizer in
    state: TokenState,
    /// End of file reached - parsing stops
    eof: bool,
    allowed_char: Option<u8>,
    pos: usize,
    /// encoding specified in the xml, or utf8 if none found
    #[cfg(feature = "encoding")]
    encoding: &'static Encoding,
    /// checks if xml5 could identify encoding
    #[cfg(feature = "encoding")]
    is_encoding_set: bool,
}

impl<'a, R: BufRead> Tokenizer<'a, R> {
    pub fn from_reader(source: R, buffer: &'a mut Vec<u8>) -> Self {
        Tokenizer::new_with_emitter(source, DefaultEmitter::default(), buffer)
    }
}

impl<'a> Tokenizer<'a, &'a [u8]> {
    pub fn from_str(s: &'a str, buffer: &'a mut Vec<u8>) -> Self {
        Tokenizer::new_with_emitter(s.as_bytes(), DefaultEmitter::default(), buffer)
    }
}

impl<'a, R: BufRead, E: Emitter<OutToken=Token>> Iterator for Tokenizer<'a, R, E> {
    type Item = Token;

    fn next(&mut self) -> Option<Token> {
        loop {
            if let Some(token) = self.emitter.pop_token() {
                break Some(token);
            } else if !self.eof {
                match self.next_state() {
                    Control::Continue => (),
                    Control::Eof => {
                        self.eof = true;
                        self.emitter.emit_eof();
                    }
                    Control::Err(e) => break Some(Token::Error(e))
                }
            } else {
                break None;
            }
        }
    }
}

pub(crate) enum Control {
    Continue,
    Eof,
    Err(Xml5Error),
}

#[derive(Debug, Clone, Copy)]
#[doc(hidden)]
enum TokenState {
    Data,
    CharRefInData(AttrValueKind),
    TagOpen,
    EndTagOpen,
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
    BogusComment,
    TagName,
    EmptyTag,
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

}

#[derive(Debug, Clone, Copy)]
#[doc(hidden)]
pub enum AttrValueKind {
    Unquoted,
    SingleQuoted,
    DoubleQuoted,
}

#[derive(Debug, Clone, Copy)]
pub enum DoctypeKind {
    Public,
    System,
}

