use std::collections::{BTreeSet, VecDeque};

use crate::errors::Xml5Error;
use crate::Token;
use crate::Token::Text;
use crate::tokenizer::DoctypeKind;

pub trait Emitter {
    type OutToken;

    fn pop_token(&mut self) -> Option<Self::OutToken>;
    fn flush_text(&mut self);

    fn create_tag(&mut self, ascii: u8);
    fn append_tag<T: AsRef<[u8]>>(&mut self, bytes: T);
    fn create_end_tag(&mut self, ascii: u8);
    fn set_empty_tag(&mut self);
    fn create_attr(&mut self, ascii: u8);
    fn push_attr_value(&mut self, ascii: u8);
    fn push_attr_values<T: AsRef<[u8]>>(&mut self, bytes: T);

    fn create_pi_tag(&mut self, ascii: u8);
    fn append_pi_target<T: AsRef<[u8]>>(&mut self, bytes: T);
    fn append_pi_data<T: AsRef<[u8]>>(&mut self, bytes: T);
    fn append_pi_data_byte(&mut self, ascii: u8);

    fn create_doctype(&mut self);
    fn append_doctype_name(&mut self, ascii: u8);
    fn append_doctype_id<T: AsRef<[u8]>>(&mut self, bytes: T);
    fn clear_doctype_id(&mut self, doctype: DoctypeKind);

    fn create_comment_token(&mut self);
    fn emit_comment(&mut self);
    fn append_to_comment<T: AsRef<[u8]>>(&mut self, bytes: T);
    fn append_to_comment_data(&mut self, ascii: u8);

    fn emit_eof(&mut self);
    fn emit_pi(&mut self);
    fn emit_error(&mut self, err: Xml5Error);
    fn emit_char(&mut self, chr: u8);
    fn emit_chars<T: AsRef<[u8]>>(&mut self, buf: T);
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
pub struct DefaultEmitter {
    current_characters: Vec<u8>,
    current_tag: Vec<u8>,
    current_token: CurrentToken,
    current_pi_target: Vec<u8>,
    current_pi_data: Vec<u8>,
    last_start_tag: Vec<u8>,
    current_attribute: Option<(Vec<u8>, Vec<u8>)>,
    seen_attributes: BTreeSet<Vec<u8>>,
    tokens_to_emit: VecDeque<Token>,
}

impl Emitter for DefaultEmitter {
    type OutToken = Token;

    fn pop_token(&mut self) -> Option<Token> {
        self.tokens_to_emit.pop_back()
    }

    fn flush_text(&mut self) {
        if !self.current_characters.is_empty() {
            let mut swap = Vec::new();
            std::mem::swap(&mut swap, &mut self.current_characters);
            self.tokens_to_emit.push_front(Text(swap));
        }
    }

    fn create_tag(&mut self, byt: u8) {
        self.current_token = CurrentToken::StartTag;
        self.current_tag.push(byt);
    }

    fn append_tag<T: AsRef<[u8]>>(&mut self, bytes: T) {
        self.current_tag.extend_from_slice(bytes.as_ref());
    }

    fn create_end_tag(&mut self, byt: u8) {
        self.current_token = CurrentToken::EndTag;
        self.current_tag.clear();
        self.current_tag.push(byt);
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

    fn push_attr_values<T: AsRef<[u8]>>(&mut self, bytes: T) {
        todo!()
    }

    fn create_pi_tag(&mut self, byt: u8) {
        self.current_token = CurrentToken::ProcessingInstruction;
        self.current_pi_target.clear();
        self.current_pi_target.push(byt);
    }

    fn append_pi_target<T: AsRef<[u8]>>(&mut self, bytes: T) {
        self.current_pi_target.extend_from_slice(bytes.as_ref());
    }

    fn append_pi_data<T: AsRef<[u8]>>(&mut self, bytes: T) {
        self.current_pi_data.extend_from_slice(bytes.as_ref());
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

    fn append_doctype_id<T: AsRef<[u8]>>(&mut self, bytes: T) {
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

    fn append_to_comment<T: AsRef<[u8]>>(&mut self, bytes: T) {
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
        if chr.is_ascii() {
            self.current_characters.push(chr as u8);
        } else {
            self.emit_chars(format!("{}", chr));
        }
    }


    fn emit_chars<T: AsRef<[u8]>>(&mut self, buf: T) {
        self.current_characters.extend_from_slice(&buf.as_ref());
    }

    fn emit_tag(&mut self) {
        todo!()
    }

    fn emit_doctype(&mut self) {
        todo!()
    }
}
