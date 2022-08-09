use std::collections::VecDeque;
use std::io;
use std::io::BufRead;

use crate::errors::Xml5Error;
use crate::tokenizer::emitter::{Appendable, CurrentToken, Emitter};
use crate::tokenizer::reader::FastRead::{Char, InterNeedle, EOF};
use crate::tokenizer::reader::{fast_find, FastRead};
use crate::Token;
#[cfg(feature = "encoding_rs")]
use encoding_rs::Encoding;

mod decoding;
mod emitter;
mod machine;
mod reader;

pub struct Tokenizer<'b, S: BufRead> {
    pub(crate) source: S,
    pub(crate) buffer: &'b mut Vec<u8>,
    /// which state is the tokenizer in
    state: TokenState,
    /// Tokens to emit
    emitted_token: VecDeque<Token<'b>>,
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

impl<'b, S: BufRead> Emitter for Tokenizer<'b, S> {
    type OutToken = Token<'b>;

    fn pop_token(&mut self) -> Option<Self::OutToken> {
        todo!()
    }

    fn flush_text(&mut self) {
        todo!()
    }

    fn create_tag(&mut self) {
        self.token_type = CurrentToken::StartTag;
    }

    fn append_tag<T: Appendable>(&mut self, appendable: T) {
        appendable.append_to_buf(&mut self.buffer, &self.source);
    }

    fn create_end_tag(&mut self) {
        todo!()
    }

    fn set_empty_tag(&mut self) {
        todo!()
    }

    fn create_attr(&mut self) {
        todo!()
    }

    fn push_attr_values<T: Appendable>(&mut self, appendable: T) {
        todo!()
    }

    fn push_attr_name<T: Appendable>(&mut self, appendable: T) {
        todo!()
    }

    fn create_pi_tag(&mut self) {
        todo!()
    }

    fn append_pi_target<T: Appendable>(&mut self, appendable: T) {
        todo!()
    }

    fn append_pi_data<T: Appendable>(&mut self, appendable: T) {
        todo!()
    }

    fn create_doctype(&mut self) {
        todo!()
    }

    fn append_doctype_name<T: Appendable>(&mut self, appendable: T) {
        todo!()
    }

    fn append_doctype_id<T: Appendable>(&mut self, appendable: T) {
        todo!()
    }

    fn clear_doctype_id(&mut self, doctype: DoctypeKind) {
        todo!()
    }

    fn create_comment_token(&mut self) {
        todo!()
    }

    fn emit_comment(&mut self) {
        todo!()
    }

    fn append_to_comment<T: Appendable>(&mut self, appendable: T) {
        todo!()
    }

    fn emit_eof(&mut self) {
        todo!()
    }

    fn emit_pi(&mut self) {
        todo!()
    }

    fn emit_error(&mut self, err: Xml5Error) {
        todo!()
    }

    fn emit_chars<T: Appendable>(&mut self, appendable: T) {
        todo!()
    }

    fn emit_tag(&mut self) {
        todo!()
    }

    fn emit_doctype(&mut self) {
        todo!()
    }
}

impl<'a, R: BufRead> Tokenizer<'a, R> {
    pub fn from_reader(source: R, buffer: &'a mut Vec<u8>) -> Self {
        Tokenizer::new_with_emitter(source, buffer)
    }
}

impl<'a> Tokenizer<'a, &'a [u8]> {
    pub fn from_str(s: &'a str, buffer: &'a mut Vec<u8>) -> Self {
        Tokenizer::new_with_emitter(s.as_bytes(), buffer)
    }
}

impl<'a, S: BufRead> Tokenizer<'a, S> {
    pub fn new_with_emitter(source: S, buffer: &'a mut Vec<u8>) -> Tokenizer<'a, S> {
        Tokenizer {
            source,
            buffer,
            eof: false,
            allowed_char: None,
            reader_pos: 0,
            current_token: (0, 0),
            state: TokenState::Data,
            token_type: Default::default(),
            emitted_token: Default::default(),
            #[cfg(feature = "encoding")]
            encoding: ::encoding_rs::UTF_8,
            #[cfg(feature = "encoding")]
            is_encoding_set: false,
        }
    }

    pub(crate) fn read_fast_until(&mut self, needle: &[u8]) -> FastRead {
        loop {
            // fill buffer
            let available = match self.source.fill_buf() {
                Ok(n) if n.is_empty() => return EOF,
                Ok(n) => n,
                Err(ref e) if e.kind() == io::ErrorKind::Interrupted => continue,
                Err(_) => return EOF,
            };

            let (read, n) = match fast_find(needle, &available[..]) {
                Some(0) => (Char(available[0]), 1),
                Some(size) => {
                    let start = self.buffer.len();
                    self.buffer.extend_from_slice(&available[..size]);
                    (InterNeedle(start, size), size)
                }
                None => (EOF, 0),
            };
            self.source.consume(n);
            return read;
        }
    }

    #[inline(always)]
    pub(crate) fn try_read_slice_exact(&mut self, needle: &str) -> bool {
        self.try_read_slice(needle, true)
    }

    pub(crate) fn try_read_slice(&mut self, needle: &str, case_sensitive: bool) -> bool {
        let mut buff: Vec<u8> = Vec::new();
        while buff.is_empty() {
            match self.source.fill_buf() {
                Ok(n) if n.is_empty() => return false,
                Ok(n) => buff.extend_from_slice(n),
                Err(ref e) if e.kind() == io::ErrorKind::Interrupted => continue,
                Err(_) => return false,
            };
        }

        if buff.len() < needle.len() {
            return false;
        }

        let read = if case_sensitive {
            buff[0..needle.len()].starts_with(needle.as_bytes())
        } else {
            for (pos, x) in needle.as_bytes().iter().enumerate() {
                if buff[pos].to_ascii_lowercase() != x.to_ascii_lowercase() {
                    false;
                }
            }
            true
        };

        if read {
            self.source.consume(needle.len());
        }
        read
    }
}

impl<'a, R: BufRead> Iterator for Tokenizer<'a, R> {
    type Item = Token<'a>;

    fn next(&mut self) -> Option<Token<'a>> {
        loop {
            if let Some(token) = self.pop_token() {
                break Some(token);
            } else if !self.eof {
                match self.next_state() {
                    Control::Continue => (),
                    Control::Eof => {
                        self.eof = true;
                        self.emit_eof();
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
