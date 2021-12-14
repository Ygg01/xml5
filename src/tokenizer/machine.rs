use std::io::BufRead;
use FastRead::EOF;

use crate::{Token, Tokenizer};
use crate::errors::{Xml5Error, Xml5Result};
use crate::errors::Xml5Error::{Eof, UnexpectedSymbol};
use crate::tokenizer::emitter::{DefaultEmitter, Emitter};
use crate::tokenizer::reader::FastRead::{InterNeedle, Needle};
use crate::tokenizer::reader::{FastRead, Reader};
use crate::tokenizer::{Control, TokenState};
use crate::tokenizer::TokenState::{Data, EndTag, EndTagNameAfter, Tag};

#[inline(always)]
pub(crate) fn is_(b: u8) -> bool {
    match b {
        b'\t' | b'\r' | b'\n' | b' ' | b':' | b'<' | b'>' => true,
        _ => false,
    }
}

impl<R: BufRead, E: Emitter> Tokenizer<R, E> {
    pub fn new_with_emitter(reader: R, emitter: E) -> Self {
        Tokenizer {
            emitter,
            reader,
            eof: false,
            reconsume_buf: vec![],
            state: TokenState::Data,
            #[cfg(feature = "encoding")]
            encoding: ::encoding_rs::UTF_8,
            #[cfg(feature = "encoding")]
            is_encoding_set: false,
        }
    }

    #[inline]
    pub(crate) fn next_state(&mut self) -> Control {
        let next_char = match self.consume_next_input() {
            Ok(c) => c,
            Err(e) => return Control::Err(e),
        };
        let mut amt = 1usize;
        match self.state {
            TokenState::Data => {
                match self.reader.read_fast_until2(b'<', b'&') {
                    Needle(b'&') => self.state = TokenState::CharRefInData,
                    Needle(b'<') => self.state = TokenState::Tag,
                    InterNeedle(text) => self.emitter.emit_chars(text),
                    _ => self.emitter.emit_eof(),
                }
            }
            TokenState::Tag => {
                match next_char {
                    Some(b'/') => self.state = TokenState::EndTag,
                    Some(b'?') => self.state = TokenState::Pi,
                    Some(b'!') => self.state = TokenState::MarkupDecl,
                    None | Some(b'\t') | Some(b'\r') | Some(b'\n')
                    | Some(b' ') | Some(b':') | Some(b'<') | Some(b'>') => {
                        self.emitter.emit_error(UnexpectedSymbol(next_char));
                        self.emitter.emit_char('<');
                        self.state = TokenState::Data;
                    }
                    Some(c) => {
                        self.emitter.create_tag(c);
                        self.state = TokenState::TagName;
                    }
                }
            }
            TokenState::EndTag => {
                match next_char {
                    Some(b'>') => {
                        self.emitter.emit_short_end_tag();
                        self.state = TokenState::Data;
                    }
                    None | Some(b'\t') | Some(b'\r') | Some(b'\n')
                    | Some(b' ') | Some(b':') | Some(b'<') => {
                        self.emitter.emit_error(Xml5Error::UnexpectedSymbol(next_char));
                        self.emitter.emit_chars("</");
                        amt = 0;
                        self.state = TokenState::Data;
                    }
                    Some(byte) => {
                        self.emitter.create_end_tag(byte);
                        self.state = TokenState::EndTagName;
                    }
                }
            }
            TokenState::EndTagName => {
                match next_char {
                    Some(b'\t') | Some(b'\r') | Some(b'\n')
                    | Some(b' ') => self.state = EndTagNameAfter,
                    Some(b'/') => {
                        self.emitter.emit_error(UnexpectedSymbol(Some(b'/')));
                        self.state = TokenState::EndTagNameAfter;
                    },
                    Some(b'>') => {
                        self.emitter.emit_token();
                        self.state = TokenState::Data;
                    },
                    Some(byte) => {
                        self.emitter.append_tag(byte);
                    },
                    None => {
                        self.emitter.emit_error(Eof);
                        self.emitter.emit_token();
                    },
                }
            }
            _ => {}
        };
        self.reader.consume(amt);
        Control::Continue
    }

    fn consume_next_input(&mut self) -> Xml5Result<Option<u8>> {
        if !self.reconsume_buf.is_empty() {
            return Ok(self.reconsume_buf.pop());
        }
        self.reader.peek_byte()
    }
}