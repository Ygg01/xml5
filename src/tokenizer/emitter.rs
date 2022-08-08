use std::collections::{BTreeSet, VecDeque};

use crate::errors::Xml5Error;
use crate::events::BytesText;
use crate::tokenizer::DoctypeKind;
use crate::Token;
use crate::Token::Text;

pub trait Emitter {
    type OutToken;

    fn pop_token(&mut self) -> Option<Self::OutToken>;
    fn flush_text(&mut self);

    fn create_tag(&mut self, ascii: u8);
    fn append_tag(&mut self, start: usize, len: usize);
    fn create_end_tag(&mut self, ascii: u8);
    fn set_empty_tag(&mut self);
    fn create_attr(&mut self, ascii: u8);
    fn push_attr_value(&mut self, ascii: u8);
    fn push_attr_values(&mut self, start: usize, len: usize);

    fn create_pi_tag(&mut self, ascii: u8);
    fn append_pi_target(&mut self, start: usize, len: usize);
    fn append_pi_data(&mut self, start: usize, len: usize);
    fn append_pi_data_byte(&mut self, ascii: u8);

    fn create_doctype(&mut self);
    fn append_doctype_name(&mut self, ascii: u8);
    fn append_doctype_id(&mut self, start: usize, len: usize);
    fn clear_doctype_id(&mut self, doctype: DoctypeKind);

    fn create_comment_token(&mut self);
    fn emit_comment(&mut self);
    fn append_to_comment(&mut self, start: usize, len: usize);
    fn append_str_to_comment(&mut self, str: &str);
    fn append_to_comment_data(&mut self, ascii: u8);

    fn emit_eof(&mut self);
    fn emit_pi(&mut self);
    fn emit_error(&mut self, err: Xml5Error);
    fn emit_char(&mut self, chr: u8);
    fn emit_chars(&mut self, start: usize, len: usize);
    fn emit_chars_str(&mut self, str: &str);
    fn emit_tag(&mut self);
    fn emit_doctype(&mut self);
}

#[derive(Copy, Clone)]
enum CurrentToken {
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

#[derive(Default)]
pub struct DefaultEmitter<'a> {
    current_characters: BytesText<'a>,
    current_tag: BytesText<'a>,
    current_token: CurrentToken,
    current_pi_target: BytesText<'a>,
    current_pi_data: BytesText<'a>,
    last_start_tag: BytesText<'a>,
    current_attribute: Option<(BytesText<'a>, BytesText<'a>)>,
    seen_attributes: BTreeSet<BytesText<'a>>,
    tokens_to_emit: VecDeque<Token<'a>>,
}

impl<'a> Emitter for DefaultEmitter<'a> {
    type OutToken = Token<'a>;

    fn pop_token(&mut self) -> Option<Token<'a>> {
        self.tokens_to_emit.pop_back()
    }

    fn flush_text(&mut self) {
        todo!()
    }

    fn create_tag(&mut self, byt: u8) {
        todo!()
    }

    fn append_tag(&mut self, start: usize, len: usize) {
        todo!()
    }

    fn create_end_tag(&mut self, byt: u8) {
        todo!()
    }

    fn set_empty_tag(&mut self) {
        todo!()
    }

    fn create_attr(&mut self, ascii: u8) {
        todo!()
    }

    fn push_attr_value(&mut self, ascii: u8) {
        todo!()
    }

    fn push_attr_values(&mut self, start: usize, len: usize) {
        todo!()
    }

    fn create_pi_tag(&mut self, byt: u8) {
        todo!()
    }

    fn append_pi_target(&mut self, start: usize, len: usize) {
        todo!()
    }

    fn append_pi_data(&mut self, start: usize, len: usize) {
        todo!()
    }

    fn append_pi_data_byte(&mut self, byt: u8) {
        todo!()
    }

    fn create_doctype(&mut self) {
        todo!()
    }

    fn append_doctype_name(&mut self, ascii: u8) {
        todo!()
    }

    fn append_doctype_id(&mut self, start: usize, len: usize) {
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

    fn append_to_comment(&mut self, start: usize, len: usize) {
        todo!()
    }

    fn append_str_to_comment(&mut self, str: &str) {
        todo!()
    }

    fn append_to_comment_data(&mut self, ascii: u8) {
        todo!()
    }

    fn emit_eof(&mut self) {
        self.tokens_to_emit.push_front(Token::Eof);
    }

    fn emit_pi(&mut self) {
        todo!()
    }

    fn emit_error(&mut self, err: Xml5Error) {
        self.tokens_to_emit.push_front(Token::Error(err));
    }

    fn emit_char(&mut self, chr: u8) {
        todo!()
    }

    fn emit_chars(&mut self, start: usize, len: usize) {
        todo!()
    }

    fn emit_chars_str(&mut self, str: &str) {
        todo!()
    }

    fn emit_tag(&mut self) {
        todo!()
    }

    fn emit_doctype(&mut self) {
        todo!()
    }
}
