use std::io::BufRead;

use crate::{Token, Tokenizer};
use crate::errors::{Xml5Error, Xml5Result};
use crate::tokenizer::emitter::{DefaultEmitter, Emitter};
use crate::tokenizer::reader::FastRead::{InterNeedle, Needle};
use crate::tokenizer::reader::{FastRead, Reader};
use crate::tokenizer::{Control, TokenState};
use crate::tokenizer::TokenState::{EndTag, Tag};

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
            Ok(None) => {
                self.eof = true;
                self.emitter.emit_eof();
                return Control::Eof;
            }
            Ok(Some(c)) => c,
            Err(e) => return Control::Err(e),
        };
        match self.state {
            TokenState::Data => {
                match self.reader.read_fast_until2(b'<', b'&') {
                    FastRead::Needle(b'&') => self.state = TokenState::CharRefInData,
                    FastRead::Needle(b'<') => self.state = TokenState::Tag,
                    FastRead::InterNeedle(text) => self.emitter.emit_chars(text),
                    _ => self.emitter.emit_eof(),
                }
            },
            TokenState::Tag => {
                match next_char {
                    b'/' => self.state = TokenState::EndTag,
                    b'?' => self.state = TokenState::Pi,
                    b'!'=> self.state = TokenState::MarkupDecl,
                    b'\t' | b'\n' | b' ' | b':' | b'<' | b'>' => {
                        // self.emitter.emit_error();
                        self.emitter.emit_char('<');
                        self.state = Tag;
                    },
                    _ => {}
                }
            },
            _ => {}
        };
        Control::Continue
    }

    fn consume_next_input(&mut self) -> Xml5Result<Option<u8>> {
        if (!self.reconsume_buf.is_empty()) {
            return Ok(self.reconsume_buf.pop());
        }
        self.reader.peek_byte()
    }
}