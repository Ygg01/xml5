use std::borrow::Cow;
use std::io::BufRead;
use std::ops::Range;
use memchr::{memchr, memchr2};
use crate::errors::Error;
use crate::events::BytesText;
use crate::events::Event::Text;
use crate::Tokenizer;
use crate::tokenizer::{TokenResult, TokenState};
use crate::tokenizer::reader::Reader;

impl<R: BufRead> Tokenizer<R> {
    pub fn from_reader(reader: R) -> Tokenizer<R> {
        Tokenizer {
            reader,
            pos: 0,
            state: TokenState::Data,
            event_ready: Text(BytesText::default()),
            current_text: vec![],
            current_tag: Range::default(),
            #[cfg(feature = "encoding")]
            encoding: ::encoding_rs::UTF_8,
            #[cfg(feature = "encoding")]
            is_encoding_set: false,
        }
    }

    #[inline]
    pub fn read_event<'s: 'b, 'b>(&'s mut self, buf: &'b mut Vec<u8>) -> TokenResult<'b>
    {
        self.read_event_buffered(buf)
    }

    fn read_event_buffered<'i, 'r, B>(&'i mut self, buf: B) -> TokenResult<'i>
        where
            R: Reader<'i, 'r, B>
    {
        loop {
            let next_char = match self.reader.read_pos(self.pos) {
                Err(e) => return self.emit_error(buf, e),
                Ok(None) => return self.emit_error(buf, Error::Eof),
                Ok(Some(chr)) => chr,
            };

            match self.state {
                TokenState::Data => {
                    match self.read_until2(b'&', b'<') {
                        b'&' => self.state = TokenState::CharRefInData,
                        b'<' => self.state = TokenState::Tag,
                        _ => self.emit_input_character(),
                    }
                }
                TokenState::CharRefInData => {
                    // TODO
                }
                TokenState::Tag => {
                    match next_char {
                        b'/' => self.state = TokenState::EndTag,
                        b'?' => self.state = TokenState::Pi,
                        b'!' => self.state = TokenState::MarkupDecl,
                        b'\t' | b'\n' | b' ' | b':' | b'<' | b'>' => {
                            self.emit_character(b'<'); // same as emitting '<' char
                            self.state = TokenState::Data;
                            self.pos -= 1; //reconsume
                            break self.emit_error(buf, Error::UnexpectedSymbol(next_char));
                        }
                        _ => {
                            self.emit_tag();
                            self.state = TokenState::TagName;
                        }
                    }
                }
                _ => (),
            }


            self.pos += 1;
        }
    }

    fn read_until2(&mut self, needle1: u8, needle2: u8) -> u8
    {
        if let Some(pos) = memchr2(needle1, needle2, &self.reader.fill_buf())
        {
            self.reader.read_pos(pos).unwrap()
        }
    }

    #[inline]
    fn emit_input_character(&mut self)
    {
        // TODO
    }

    #[inline]
    fn emit_character(&mut self, chr: u8)
    {
        // TODO
    }

    fn emit_tag(&mut self)
    {}

    fn emit_error<'i, 'r, B>(&'i mut self, buf: B, err: Error) -> TokenResult<'i>
        where
            R: Reader<'i, 'r, B>
    {
        let error = err;
        let ev = match &self.event_ready {
            Text(_) => {
                todo!()
            }
            _ => Text(BytesText::default())
        };
        TokenResult {
            event: ev,
            error,
        }
    }
}