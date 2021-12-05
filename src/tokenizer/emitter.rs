use std::collections::VecDeque;
use crate::Token;

pub trait Emitter {
    type Token;

    fn pop_token(&mut self) -> Option<Self::Token>;
    fn emit_eof(&mut self);
}

pub struct DefaultEmitter {
}

impl Emitter for DefaultEmitter {
    type Token = Token;

    fn pop_token(&mut self) -> Option<Token> {
        Some(Token::Eof)
    }

    fn emit_eof(&mut self) {
        todo!()
    }
}

impl Default for DefaultEmitter {
    fn default() -> Self {
        DefaultEmitter {
        }
    }
}