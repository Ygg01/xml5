use std::collections::VecDeque;
use crate::Token;

pub trait Emitter {
    type Token;

    fn pop_token(&mut self) -> Option<Self::Token>;
}

pub struct DefaultEmitter {
}

impl Emitter for DefaultEmitter {
    type Token = Token;

    fn pop_token(&mut self) -> Option<Token> {
        Some(Token::Eof)
    }
}

impl Default for DefaultEmitter {
    fn default() -> Self {
        DefaultEmitter {
        }
    }
}