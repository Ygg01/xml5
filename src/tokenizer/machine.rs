use std::borrow::Cow;
use std::io::BufRead;
use std::ops::Range;
use memchr::{memchr, memchr2};
use crate::errors::{Xml5Error, Xml5Result};
use crate::events::Event::Text;
use crate::{Event, Tokenizer};
use crate::errors::Xml5Error::Eof;
use crate::events::EmitEvent;
use crate::tokenizer::{TokenState};
use crate::tokenizer::reader::FastRead::{InterNeedle, Needle};
use crate::tokenizer::reader::Reader;

impl<R: BufRead> Tokenizer<R> {
    pub fn from_reader(reader: R) -> Tokenizer<R> {
        Tokenizer {
            reader,
            pos: 0,
            state: TokenState::Data,
            events_to_emit: vec![],
            current_text: vec![],
            current_tag: vec![],
            #[cfg(feature = "encoding")]
            encoding: ::encoding_rs::UTF_8,
            #[cfg(feature = "encoding")]
            is_encoding_set: false,
        }
    }

    #[inline]
    pub fn read_event(&mut self) -> Xml5Result<Event>
    {
        loop {
            let next_char = match self.reader.peek_byte()?
            {
                Some(next_char) => next_char,
                None => return self.emit_error(Xml5Error::Eof),
            };
            match self.state {
                TokenState::Data => {
                    match self.reader.read_fast_until2(b'<', b'/')? {
                        Needle(b'&') => self.state = TokenState::CharRefInData,
                        Needle(b'<') => self.state = TokenState::Tag,
                        Needle(_) => self.emit_eof(),
                        InterNeedle(txt) => {
                            self.emit_input_characters(txt);
                        },

                    }
                },
                TokenState::Tag => {
                    match next_char {
                        b'/' => self.state = TokenState::EndTag,
                        b'?' => self.state = TokenState::Pi,
                        b'!' => self.state = TokenState::MarkupDecl,
                        b'\t' | b'\n' | b' ' | b':' | b'<' | b'>' => {
                            self.emit_character(b'<'); // same as emitting '<' char
                            self.state = TokenState::Data;
                            self.pos -= 1; //reconsume
                            break self.emit_error(Xml5Error::Eof);
                        },
                        _ => {},
                    }
                }
                _ => {}
            }
        }
    }

    #[inline]
    fn emit_input_characters(&mut self, mut buf: Vec<u8>)
    {
        self.current_text.append(&mut buf)
    }

    #[inline]
    fn emit_eof(&mut self)
    {
        self.emit_error(Xml5Error::Eof);
    }

    #[inline]
    fn emit_character(&mut self, chr: u8)
    {
        if Some(&EmitEvent::Text) == self.events_to_emit.last() {
            self.current_text.push(chr);
        } else {
            self.events_to_emit.push(EmitEvent::Text);
        }
    }

    fn emit_tag(&mut self)
    {}

    #[inline ]
    fn emit_error(&mut self, error: Xml5Error) -> Xml5Result<Event>
    {
        todo!()
    }
}