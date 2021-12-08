use std::collections::{BTreeSet, VecDeque};
use crate::errors::Xml5Error;
use crate::Token;

pub trait Emitter {
    type Token;

    fn pop_token(&mut self) -> Option<Self::Token>;
    fn emit_eof(&mut self);
    fn emit_error(&mut self, err: Xml5Error);
    fn emit_chars<T: AsRef<[u8]>>(&mut self, buf: T);
    fn emit_char(&mut self, chr: char);
}

#[derive(Default)]
pub struct DefaultEmitter {
    current_characters: Vec<u8>,
    current_token: Option<Token>,
    last_start_tag: Vec<u8>,
    current_attribute: Option<(Vec<u8>, Vec<u8>)>,
    seen_attributes: BTreeSet<Vec<u8>>,
    emitted_tokens: VecDeque<Token>,
}

impl Emitter for DefaultEmitter {
    type Token = Token;

    fn pop_token(&mut self) -> Option<Token> {
        self.emitted_tokens.pop_back()
    }

    fn emit_eof(&mut self) {
        self.emitted_tokens.push_front(Token::Eof);
    }

    fn emit_error(&mut self, err: Xml5Error) {
        self.emitted_tokens.push_front(Token::Error(err));
    }

    fn emit_chars<T: AsRef<[u8]>>(&mut self, buf: T) {
        self.current_characters.extend_from_slice(&buf.as_ref());
    }

    fn emit_char(&mut self, chr: char) {
        if (chr.is_ascii()){
            self.current_characters.push(chr as u8);
        } else {
            self.emit_chars(format!("{}", chr));
        }
    }
}
