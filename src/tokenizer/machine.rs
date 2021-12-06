use std::io::BufRead;

use crate::{Token, Tokenizer};
use crate::errors::{Xml5Error, Xml5Result};
use crate::tokenizer::emitter::{DefaultEmitter, Emitter};
use crate::tokenizer::reader::FastRead::{InterNeedle, Needle};
use crate::tokenizer::reader::{FastRead, Reader};
use crate::tokenizer::{Control, TokenState};

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
        let char = match self.consume_next_input() {
            Ok(None) => {
                self.eof = true;
                self.emitter.emit_eof();
                return Control::Eof;
            }
            Ok(Some(c)) => c,
            Err(e) => return Control::Err(e),
        };
        match self.state {
            TokenState::Data => {}
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