use std::collections::VecDeque;
use crate::errors::Xml5Error;
use crate::Token;

pub trait Emitter {
    type Token;

    fn pop_token(&mut self) -> Option<Self::Token>;
    fn emit_eof(&mut self);
    fn emit_error(&mut self, err: Xml5Error);
    fn emit_chars(&mut self, buf: Vec<u8>);
    fn emit_char(&mut self, chr: char);
}

pub struct DefaultEmitter {
    emit_tokens: VecDeque<Token>,
}

impl Default for DefaultEmitter {
    fn default() -> Self {
        DefaultEmitter {
            emit_tokens: VecDeque::new(),
        }
    }
}

impl Emitter for DefaultEmitter {
    type Token = Token;

    fn pop_token(&mut self) -> Option<Token> {
        Some(Token::Eof)
    }

    fn emit_eof(&mut self) {
        todo!()
    }

    fn emit_error(&mut self, err: Xml5Error) {
        todo!()
    }

    fn emit_chars(&mut self, buf: Vec<u8>) {
        todo!()
    }

    fn emit_char(&mut self, chr: char) {
        todo!()
    }
}
