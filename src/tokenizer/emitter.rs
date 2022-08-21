use std::collections::VecDeque;
use std::ops::Range;

use crate::errors::Xml5Error;
use crate::tokenizer::emitter::Spans::{Characters, Span};
use crate::tokenizer::DoctypeKind;
use crate::{Token, Tokenizer};

pub trait Emitter {
    type Output;

    fn pop_token(&mut self) -> Option<Self::Output>;
    fn emit_current_token(&mut self);

    fn create_tag(&mut self);
    fn append_tag(&mut self, start: usize, end: usize);
    fn create_end_tag(&mut self);
    fn set_empty_tag(&mut self);
    fn create_attr(&mut self);
    fn attr_values(&mut self, start: usize, end: usize);
    fn attr_names(&mut self, start: usize, end: usize);

    fn create_pi_tag(&mut self);
    fn pi_data(&mut self, start: usize, end: usize);
    fn pi_target(&mut self, start: usize, end: usize);

    fn create_doctype(&mut self);
    fn doctype_id(&mut self, start: usize, end: usize);
    fn doctype_name(&mut self, start: usize, end: usize);
    fn doctype_name_now(&mut self, chr: u8);
    fn clear_doctype_id(&mut self, doctype: DoctypeKind);

    fn create_comment_token(&mut self);
    fn emit_comment(&mut self);
    fn to_comment(&mut self, start: usize, end: usize);
    fn to_comment_now<T: IntoBytes>(&mut self, x: T);

    fn emit_eof(&mut self);
    fn emit_pi(&mut self);
    fn emit_error(&mut self, err: Xml5Error);
    fn emit_chars(&mut self, start: usize, end: usize);
    fn emit_chars_now<T: IntoBytes>(&mut self, x: T);
    fn emit_tag(&mut self);
    fn emit_doctype(&mut self);
}

pub trait IntoBytes {
    // fn to_bytes(&self) -> [u8];
}

impl IntoBytes for u8 {
    // fn to_bytes(&self) -> &[u8] {
    //     *self.to_be_bytes()
    // }
}

impl IntoBytes for &str {
    // fn to_bytes(&self) -> &[u8] {
    //     self.as_bytes()
    // }
}

#[derive(Default)]
pub struct DefaultEmitter {
    tokens: VecDeque<SpanTokens>,
    current_token_type: CurrentToken,
    current_token_bounds: Range<usize>,
    current_token_secondary_bound: Range<usize>,
    current_text: Vec<u8>,
}

pub enum SpanTokens {
    Text(Spans),
    CData(Spans),
    Decl(Spans),
    Comment(Spans),
    EndTag(Option<Spans>),
    PiData {
        name: Spans,
        value: Spans,
    },
    StartTag {
        self_close: bool,
        name: Spans,
        attr: Vec<(Spans, Spans)>,
    },
    Error(Xml5Error),
    Eof,
}

pub enum Spans {
    Span(Range<usize>),
    Characters(Vec<u8>),
}

impl DefaultEmitter {
    #[inline(always)]
    fn close_span(&mut self) {
        self.current_token_bounds.start = self.current_token_bounds.end;
    }

    #[inline(always)]
    fn close_span_secondary(&mut self) {
        self.current_token_secondary_bound.start = self.current_token_secondary_bound.end;
    }
}

impl Emitter for DefaultEmitter {
    type Output = SpanTokens;

    fn pop_token(&mut self) -> Option<SpanTokens> {
        self.tokens.pop_front()
    }

    fn emit_current_token(&mut self) {
        let span = if !self.current_text.is_empty() {
            let sp = Characters(self.current_text.clone());
            sp
        } else if !self.current_token_bounds.is_empty() {
            Span(self.current_token_bounds.clone())
        } else {
            // No span skip emitting
            return;
        };
        let tok = match self.current_token_type {
            CurrentToken::EndTag => SpanTokens::EndTag(Some(span)),
            _ => SpanTokens::Eof,
        };
        self.tokens.push_back(tok);
        self.close_span();
    }

    fn create_tag(&mut self) {}

    fn append_tag(&mut self, start: usize, end: usize) {
        if self.current_token_bounds.start == start {
            self.current_token_bounds.end = end;
        } else {
            self.emit_current_token();
            self.current_token_bounds.start = start;
            self.current_token_bounds.end = end;
        }
    }

    fn create_end_tag(&mut self) {
        self.current_token_type = CurrentToken::EndTag;
        self.close_span();
    }

    fn set_empty_tag(&mut self) {
        todo!()
    }

    fn create_attr(&mut self) {
        todo!()
    }

    fn attr_values(&mut self, start: usize, end: usize) {
        todo!()
    }

    fn attr_names(&mut self, start: usize, end: usize) {
        todo!()
    }

    fn create_pi_tag(&mut self) {
        self.current_token_type = CurrentToken::ProcessingInstruction;
        self.close_span();
        self.close_span_secondary();
    }

    fn pi_data(&mut self, start: usize, end: usize) {
        if self.current_token_bounds.start == start {
            self.current_token_bounds.end = end;
        } else {
            self.emit_current_token();
            self.current_token_bounds.start = start;
            self.current_token_bounds.end = end;
        }
    }

    fn pi_target(&mut self, start: usize, end: usize) {
        if self.current_token_secondary_bound.start == start {
            self.current_token_secondary_bound.end = end;
        } else {
            self.emit_current_token();
            self.current_token_secondary_bound.start = start;
            self.current_token_secondary_bound.end = end;
        }
    }

    fn create_doctype(&mut self) {
        todo!()
    }

    fn doctype_id(&mut self, start: usize, end: usize) {
        todo!()
    }

    fn doctype_name(&mut self, start: usize, end: usize) {
        todo!()
    }

    fn doctype_name_now(&mut self, chr: u8) {
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

    fn to_comment(&mut self, start: usize, end: usize) {
        todo!()
    }

    fn to_comment_now<T: IntoBytes>(&mut self, x: T) {
        todo!()
    }

    fn emit_eof(&mut self) {
        todo!()
    }

    fn emit_pi(&mut self) {
        self.tokens.push_back(SpanTokens::PiData {
            name: Span(self.current_token_bounds.clone()),
            value: Span(self.current_token_secondary_bound.clone()),
        });
    }

    fn emit_error(&mut self, err: Xml5Error) {
        self.tokens.push_back(SpanTokens::Error(err));
    }

    fn emit_chars(&mut self, start: usize, end: usize) {
        todo!()
    }

    fn emit_chars_now<T: IntoBytes>(&mut self, x: T) {
        todo!()
    }

    fn emit_tag(&mut self) {
        todo!()
    }

    fn emit_doctype(&mut self) {
        todo!()
    }
}

#[derive(Copy, Clone)]
pub enum CurrentToken {
    NoToken,
    StartTag,
    EndTag,
    ProcessingInstruction,
    Doctype,
}

impl Default for CurrentToken {
    fn default() -> Self {
        CurrentToken::NoToken
    }
}

#[repr(u32)]
pub enum Test {
    Characters(String),
    Test(u8),
}

fn main() {
    println!("{}", std::mem::size_of::<Test>());
}
