use std::collections::VecDeque;
use std::mem;
use std::ops::Range;

use crate::errors::Xml5Error;
use crate::tokenizer::DoctypeKind;
use crate::Tokenizer;

pub trait Emitter {
    type Output;

    fn pop_token(&mut self) -> Option<Self::Output>;
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
    fn emit_end_tag(&mut self);
    fn emit_tag(&mut self);
    fn emit_doctype(&mut self);

    fn set_xml_declaration(&mut self, attr_name: XmlDeclAttr);
    fn emit_decl_value(&mut self, start: usize, end: usize);
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum XmlDeclAttr {
    Version,
    Encoding,
    Standalone,
}

pub trait IntoBytes {
    fn to_bytes(&self) -> Vec<u8>;
}

impl IntoBytes for u8 {
    fn to_bytes(&self) -> Vec<u8> {
        vec![*self]
    }
}

impl IntoBytes for &str {
    fn to_bytes(&self) -> Vec<u8> {
        self.as_bytes().to_vec()
    }
}

#[derive(Default)]
pub struct DefaultEmitter {
    tokens: VecDeque<SpanTokens>,
    current_token_type: CurrentToken,
    current_token_bounds: Spans,
    encoding: Spans,
    current_token_secondary_bound: Spans,
    current_attrs: Vec<(Spans, Spans)>,
}

pub enum SpanTokens {
    Text(Spans),
    CData(Spans),
    Decl(Spans),
    Comment(Spans),
    EndTag(Option<Spans>),
    PiData {
        target: Spans,
        data: Spans,
    },
    StartTag {
        self_close: bool,
        name: Spans,
        attrs: Vec<(Spans, Spans)>,
    },
    Error(Xml5Error),
    Eof,
}

#[derive(Default, Debug)]
pub struct Spans {
    pub(crate) data: Vec<Mix>,
}

impl Spans {
    #[inline]
    pub fn to_range(&self) -> Option<(usize, usize)> {
        if self.data.len() == 1 {
            if let Some(Mix::Range(start, end)) = self.data.first() {
                return Some((*start, *end));
            }
        };
        None
    }

    #[inline(always)]
    pub fn add_span(&mut self, start: usize, end: usize) {
        if let Some(Mix::Range(_, r2)) = self.data.last_mut() {
            if &start == r2 {
                *r2 = end;
            }
        } else {
            self.data.push(Mix::Range(start, end));
        }
    }
}

#[derive(Debug)]
pub enum Mix {
    Range(usize, usize),
    Owned(Vec<u8>),
}

impl Mix {
    #[inline(always)]
    pub const fn is_borrowed(&self) -> bool {
        match self {
            Mix::Owned(_) => true,
            _ => false,
        }
    }
}

impl Emitter for DefaultEmitter {
    type Output = SpanTokens;

    fn pop_token(&mut self) -> Option<SpanTokens> {
        self.tokens.pop_front()
    }

    fn create_tag(&mut self) {}

    fn append_tag(&mut self, start: usize, end: usize) {
        self.current_token_bounds.add_span(start, end);
    }

    fn create_end_tag(&mut self) {
        self.current_token_type = CurrentToken::EndTag;
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
    }

    fn pi_data(&mut self, start: usize, end: usize) {
        self.current_token_bounds.add_span(start, end);
    }

    fn pi_target(&mut self, start: usize, end: usize) {
        self.current_token_secondary_bound.add_span(start, end);
    }

    fn create_doctype(&mut self) {
        self.current_token_type = CurrentToken::Doctype;
    }

    fn doctype_id(&mut self, start: usize, end: usize) {
        self.current_token_bounds.add_span(start, end);
    }

    fn doctype_name(&mut self, start: usize, end: usize) {
        self.current_token_secondary_bound.add_span(start, end);
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
        self.tokens.push_back(SpanTokens::Eof);
    }

    fn emit_pi(&mut self) {
        self.tokens.push_back(SpanTokens::PiData {
            data: mem::take(&mut self.current_token_bounds),
            target: mem::take(&mut self.current_token_secondary_bound),
        });
    }

    fn emit_error(&mut self, err: Xml5Error) {
        self.tokens.push_back(SpanTokens::Error(err));
    }

    fn emit_chars(&mut self, start: usize, end: usize) {
        self.tokens.push_back(SpanTokens::Text(Spans {
            data: vec![Mix::Range(start, end)],
        }));
    }

    fn emit_chars_now<T: IntoBytes>(&mut self, x: T) {
        todo!()
    }

    fn emit_end_tag(&mut self) {
        self.tokens.push_back(SpanTokens::EndTag(Some(mem::take(
            &mut self.current_token_bounds,
        ))));
    }

    fn emit_tag(&mut self) {
        // TODO add attributes
        self.tokens.push_back(SpanTokens::StartTag {
            name: mem::take(&mut self.current_token_bounds),
            attrs: mem::take(&mut self.current_attrs),
            self_close: false,
        });
    }

    fn emit_doctype(&mut self) {
        // self.tokens.push_back(SpanTokens::D)
    }

    fn set_xml_declaration(&mut self, attr_name: XmlDeclAttr) {
        todo!()
    }

    fn emit_decl_value(&mut self, start: usize, end: usize) {
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
    println!("{}", mem::size_of::<Test>());
}
