use std::collections::VecDeque;
use std::io;
use std::io::BufRead;

#[cfg(feature = "encoding_rs")]
use encoding_rs::Encoding;

use crate::errors::Xml5Error;
use crate::tokenizer::emitter::{Appendable, CurrentToken, Emitter};
use crate::tokenizer::reader::FastRead::{Char, InterNeedle, EOF};
use crate::tokenizer::reader::{fast_find, BuffReader, FastRead};
use crate::Token;

mod decoding;
mod emitter;
mod machine;
mod reader;

pub struct Tokenizer {
    /// which state is the tokenizer in
    state: TokenState,
    /// Which is current token
    token_type: CurrentToken,
    current_token: (usize, usize),
    /// End of file reached - parsing stops
    eof: bool,
    allowed_char: Option<u8>,
    reader_pos: usize,
    /// encoding specified in the xml, or utf8 if none found
    #[cfg(feature = "encoding")]
    encoding: &'static Encoding,
    /// checks if xml5 could identify encoding
    #[cfg(feature = "encoding")]
    is_encoding_set: bool,
}

pub(crate) struct BufIterator<'a, R, E> {
    pub state: Tokenizer,
    pub reader: BuffReader<'a, R>,
    pub emitter: E,
}

impl<'a, R, E> Iterator for BufIterator<'a, R, E>
where
    R: BufRead,
    E: Emitter<OutToken = Token<'a>>,
{
    type Item = Token<'a>;

    fn next(&mut self) -> Option<Token<'a>> {
        loop {
            if let Some(token) = self.emitter.pop_token() {
                break Some(token);
            } else if !self.state.eof {
                match self.state.next_state(&mut self.reader, &mut self.emitter) {
                    Control::Continue => (),
                    Control::Eof => {
                        self.state.eof = true;
                        self.emitter.emit_eof();
                    }
                    Control::Err(e) => break Some(Token::Error(e)),
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
