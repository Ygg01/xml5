use std::io::BufRead;

use crate::errors::Xml5Error;
use crate::tokenizer::DoctypeKind;

pub trait Emitter {
    type OutToken;

    fn pop_token(&mut self) -> Option<Self::OutToken>;
    fn flush_text(&mut self);

    fn create_tag(&mut self);
    fn append_tag<T: Appendable>(&mut self, appendable: T);
    fn create_end_tag(&mut self);
    fn set_empty_tag(&mut self);
    fn create_attr(&mut self);
    fn push_attr_values<T: Appendable>(&mut self, appendable: T);
    fn push_attr_name<T: Appendable>(&mut self, appendable: T);

    fn create_pi_tag(&mut self);
    fn append_pi_target<T: Appendable>(&mut self, appendable: T);
    fn append_pi_data<T: Appendable>(&mut self, appendable: T);

    fn create_doctype(&mut self);
    fn append_doctype_name<T: Appendable>(&mut self, appendable: T);
    fn append_doctype_id<T: Appendable>(&mut self, appendable: T);
    fn clear_doctype_id(&mut self, doctype: DoctypeKind);

    fn create_comment_token(&mut self);
    fn emit_comment(&mut self);
    fn append_to_comment<T: Appendable>(&mut self, appendable: T);

    fn emit_eof(&mut self);
    fn emit_pi(&mut self);
    fn emit_error(&mut self, err: Xml5Error);
    fn emit_chars<T: Appendable>(&mut self, appendable: T);
    fn emit_tag(&mut self);
    fn emit_doctype(&mut self);
}

pub trait Appendable {
    fn append_to_buf<B: BufRead>(&self, buf: &mut Vec<u8>, reader: &B);
}

impl Appendable for (usize, usize) {
    fn append_to_buf<B: BufRead>(&self, buf: &mut Vec<u8>, reader: &B) {}
}

impl Appendable for &str {
    fn append_to_buf<B: BufRead>(&self, buf: &mut Vec<u8>, reader: &B) {
        buf.extend_from_slice(self.as_bytes());
    }
}

impl Appendable for &[u8] {
    fn append_to_buf<B: BufRead>(&self, buf: &mut Vec<u8>, reader: &B) {
        buf.extend_from_slice(self);
    }
}

impl Appendable for u8 {
    fn append_to_buf<B: BufRead>(&self, buf: &mut Vec<u8>, reader: &B) {
        buf.push(*self);
    }
}

#[derive(Copy, Clone)]
pub(crate) enum CurrentToken {
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
