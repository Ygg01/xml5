use std::io;
use std::io::BufRead;
use std::num::IntErrorKind::Empty;
use std::str::from_utf8;

use FastRead::EOF;

use crate::{Token, Tokenizer};
use crate::errors::{Xml5Error, Xml5Result};
use crate::Token::Error;
use crate::tokenizer::{Control, TokenState};
use crate::tokenizer::AttrValueKind::{DoubleQuoted, SingleQuoted, Unquoted};
use crate::tokenizer::Control::Eof;
use crate::tokenizer::DoctypeKind::{Public, System};
use crate::tokenizer::emitter::{DefaultEmitter, Emitter};
use crate::tokenizer::reader::{BufferedInput, fast_find, FastRead};
use crate::tokenizer::reader::FastRead::{Char, InterNeedle};
use crate::tokenizer::TokenState::{*};

#[inline(always)]
pub(crate) fn is_ws(b: u8) -> bool {
    match b {
        b'\t' | b'\r' | b'\n' | b' ' | b':' | b'<' | b'>' => true,
        _ => false,
    }
}

impl<'a, S: BufRead, E: Emitter> Tokenizer<'a, S, E> {
    pub fn new_with_emitter(source: S, emitter: E, buffer: &'a mut Vec<u8>) -> Tokenizer<'a, S, E> {
        Tokenizer {
            emitter,
            source,
            buffer,
            eof: false,
            allowed_char: None,
            pos: 0,
            state: Data,
            #[cfg(feature = "encoding")]
            encoding: ::encoding_rs::UTF_8,
            #[cfg(feature = "encoding")]
            is_encoding_set: false,
        }
    }

    pub(crate) fn read_fast_until(&mut self, needle: &[u8]) -> FastRead {
        loop {
            // fill buffer
            let available = match self.source.fill_buf() {
                Ok(n) if n.is_empty() => return EOF,
                Ok(n) => n,
                Err(ref e) if e.kind() == io::ErrorKind::Interrupted => continue,
                Err(_) => return EOF,
            };

            let (read, n) = match fast_find(needle, &available[..]) {
                Some(0) => (Char(available[0]), 1),
                Some(size) => (InterNeedle(available[..size].to_owned()), size),
                None => (EOF, 0),
            };
            self.source.consume(n);
            return read;
        }
    }

    #[inline(always)]
    pub(crate) fn try_read_slice_exact(&mut self, needle: &str) -> bool {
        self.try_read_slice(needle, true)
    }

    pub(crate) fn try_read_slice(&mut self, needle: &str, case_sensitive: bool) -> bool {
        let mut buff: Vec<u8> = Vec::new();
        while buff.is_empty() {
            match self.source.fill_buf() {
                Ok(n) if n.is_empty() => return false,
                Ok(n) => buff.extend_from_slice(n),
                Err(ref e) if e.kind() == io::ErrorKind::Interrupted => continue,
                Err(_) => return false,
            };
        }

        if buff.len() < needle.len() {
            return false;
        }

        let read = if case_sensitive {
            buff[0..needle.len()].starts_with(needle.as_bytes())
        } else {
            for (pos, x) in needle.as_bytes().iter().enumerate()
            {
                if buff[pos].to_ascii_lowercase() != x.to_ascii_lowercase() {
                    false;
                }
            }
            true
        };

        if read {
            self.source.consume(needle.len());
        }
        read
    }

    #[inline]
    pub(crate) fn next_state(&mut self) -> Control {
        let mut amt = 1;
        let next_char = match self.source.peek_byte() {
            Ok(None) => {
                self.eof = true;
                return Eof;
            }
            Ok(x) => x,
            Err(e) => return Control::Err(e)
        };

        macro_rules! switch_to {
            ($state:expr) => { self.state = $state };
        }

        macro_rules! reconsume {
            ($state:expr) => {
                {amt= 0; self.state = $state;}
            };
        }



        match self.state {
            Data => {
                match self.read_fast_until(&[b'<', b'&']) {
                    Char(b'&') => switch_to!(CharRefInData(Unquoted)),
                    Char(b'<') => switch_to!( TagOpen),
                    InterNeedle(text) => self.emitter.emit_chars(text),
                    _ => self.emitter.emit_eof(),
                }
            }
            TagOpen => {
                match next_char {
                    Some(b'/') => switch_to!(EndTagOpen),
                    Some(b'?') => switch_to!(Pi),
                    Some(b'!') => switch_to!(MarkupDecl),
                    None | Some(b'\t') | Some(b'\n')
                    | Some(b' ') | Some(b':') | Some(b'<') | Some(b'>') => {
                        self.emitter.emit_error(Xml5Error::UnexpectedSymbolOrEof(next_char));
                        self.emitter.emit_char(b'<');
                        switch_to!(Data);
                    }
                    Some(c) => {
                        self.emitter.create_tag(c);
                        switch_to!(TagName);
                    }
                }
            }
            EndTagOpen => {
                match next_char {
                    Some(b'>') => {
                        self.emitter.emit_tag();
                        switch_to!(Data);
                    }
                    None | Some(b'\t') | Some(b'\n')
                    | Some(b' ') | Some(b':') | Some(b'<') => {
                        self.emitter.emit_error(Xml5Error::UnexpectedSymbolOrEof(next_char));
                        self.emitter.emit_chars("</");
                        reconsume!(Data);
                    }
                    Some(byte) => {
                        self.emitter.create_end_tag(byte);
                        switch_to!(EndTagName);
                    }
                }
            }
            EndTagName => {
                match self.read_fast_until(&[b'\t', b'\n', b' ', b'/', b'>']) {
                    Char(b'\t') | Char(b'\n') | Char(b' ') => {
                        switch_to!(EndTagNameAfter);
                    }
                    Char(b'/') => {
                        self.emitter.emit_error(Xml5Error::UnexpectedSymbol('/'));
                        switch_to!(EndTagNameAfter);
                    }
                    Char(b'>') => {
                        self.emitter.emit_char(b'>');
                        switch_to!(Data);
                    }
                    InterNeedle(x) => self.emitter.append_tag(x),
                    _ => {
                        self.emitter.emit_error(Xml5Error::UnexpectedEof);
                    }
                }
            }
            EndTagNameAfter => {
                match next_char {
                    Some(b'>') => {
                        self.emitter.emit_tag();
                        switch_to!(Data);
                    }
                    Some(b' ') | Some(b'\n') | Some(b'\t') => {}
                    None => {
                        self.emitter.emit_error(Xml5Error::UnexpectedSymbolOrEof(None));
                        reconsume!(Data);
                    }
                    Some(x) => {
                        self.emitter.emit_error(Xml5Error::UnexpectedSymbol(x as char));
                    }
                }
            }
            TagName => {
                match self.read_fast_until(&[b'\t', b'\n', b' ', b'>', b'/']) {
                    Char(b'\t') | Char(b'\n') | Char(b' ') => switch_to!(TagAttrNameBefore),
                    Char(b'>') => {
                        self.emitter.emit_tag();
                        switch_to!(Data);
                    }
                    Char(b'/') => {
                        self.emitter.set_empty_tag();
                        switch_to!(EmptyTag);
                    }
                    InterNeedle(buf) => {
                        self.emitter.append_tag(buf);
                    }
                    _ => {
                        self.emitter.emit_error(Xml5Error::EofInTag);
                        self.emitter.emit_tag();
                        reconsume!(Data);
                    }
                }
            }
            EmptyTag => {
                match next_char {
                    Some(b'>') => {
                        self.emitter.emit_tag();
                        switch_to!(Data);
                    }
                    _ => reconsume!(TagAttrValueBefore)
                }
            }
            TagAttrNameBefore => {
                match next_char {
                    Some(b't') | Some(b't') | Some(b't') => (),
                    Some(b'>') => {
                        self.emitter.emit_tag();
                        switch_to!(Data);
                    }
                    Some(b'/') => {
                        self.emitter.set_empty_tag();
                        switch_to!(EmptyTag);
                    }
                    Some(b':') => self.emitter.emit_error(Xml5Error::ColonBeforeAttrName),
                    None => {
                        self.emitter.emit_error(Xml5Error::EofInTag);
                        self.emitter.emit_tag();
                        reconsume!(Data);
                    }
                    Some(bit) => {
                        self.emitter.create_attr(bit);
                        switch_to!(TagAttrName);
                    }
                }
            }
            TagAttrName => {
                match self.read_fast_until(&[b'\t', b'\n', b' ', b'=', b'>', b'/']) {
                    Char(b'\t') | Char(b'\n') | Char(b' ') => switch_to!(TagAttrValueBefore),
                    Char(b'=') => switch_to!(TagAttrValueBefore),
                    Char(b'>') => {
                        self.emitter.emit_tag();
                        switch_to!(Data);
                    }
                    Char(b'/') => {
                        self.emitter.set_empty_tag();
                        switch_to!(EmptyTag);
                    }
                    EOF | _ => {
                        self.emitter.emit_error(Xml5Error::EofInTag);
                        self.emitter.emit_tag();
                        reconsume!(Data);
                    }
                }
            }
            TagAttrNameAfter => {
                match next_char {
                    Some(b'\t') | Some(b'\n') | Some(b' ') => (),
                    Some(b'=') => switch_to!(TagAttrValueBefore),
                    Some(b'>') => {
                        self.emitter.emit_tag();
                        switch_to!(EmptyTag);
                    }
                    Some(b'/') => {
                        self.emitter.set_empty_tag();
                        switch_to!(EmptyTag);
                    }
                    None => {
                        self.emitter.emit_error(Xml5Error::EofInTag);
                        self.emitter.emit_tag();
                        reconsume!(Data);
                    }
                    Some(byt) => {
                        self.emitter.create_attr(byt);
                        switch_to!(TagAttrName)
                    }
                }
            }
            TagAttrValueBefore => {
                match next_char {
                    Some(b'\t') | Some(b'\n') | Some(b' ') => (),
                    Some(b'"') => switch_to!(TagAttrValue(DoubleQuoted)),
                    Some(b'\'') => switch_to!(TagAttrValue(SingleQuoted)),
                    Some(b'&') => reconsume!(TagAttrValue(Unquoted)),
                    Some(b'>') => {
                        self.emitter.emit_tag();
                        switch_to!(Data);
                    }
                    None => {
                        self.emitter.emit_error(Xml5Error::EofInTag);
                        self.emitter.emit_tag();
                        reconsume!(Data);
                    }
                    Some(byt) => {
                        self.emitter.push_attr_value(byt);
                        switch_to!(TagAttrValue(Unquoted));
                    }
                }
            }
            TagAttrValue(DoubleQuoted) => {
                match self.read_fast_until(&[b'&', b'"']) {
                    Char(b'"') => switch_to!(TagAttrNameBefore),
                    Char(_) => switch_to!(CharRefInData(DoubleQuoted)),
                    InterNeedle(buf) => self.emitter.push_attr_values(buf),
                    EOF => {
                        self.emitter.emit_error(Xml5Error::EofInTag);
                        self.emitter.emit_tag();
                        reconsume!(Data);
                    }
                }
            }
            TagAttrValue(SingleQuoted) => {
                match self.read_fast_until(&[b'&', b'\'']) {
                    Char(b'\'') => switch_to!(TagAttrNameBefore),
                    Char(_) => switch_to!(CharRefInData(DoubleQuoted)),
                    InterNeedle(buf) => self.emitter.push_attr_values(buf),
                    EOF => {
                        self.emitter.emit_error(Xml5Error::EofInTag);
                        self.emitter.emit_tag();
                        reconsume!(Data);
                    }
                }
            }
            TagAttrValue(Unquoted) => {
                match self.read_fast_until(&[b'\t', b'\n', b' ', b'&', b'>']) {
                    Char(b'\t') | Char(b'\n') | Char(b' ') => switch_to!(TagAttrNameBefore),
                    Char(b'&') => switch_to!(CharRefInData(Unquoted)),
                    Char(_) => {
                        self.emitter.emit_tag();
                        switch_to!(Data);
                    }
                    InterNeedle(buf) => self.emitter.push_attr_values(buf),
                    EOF => {
                        self.emitter.emit_error(Xml5Error::EofInTag);
                        self.emitter.emit_tag();
                        reconsume!(Data);
                    }
                }
            }
            Pi => {
                match next_char {
                    None | Some(b' ') | Some(b'\n') | Some(b'\t') => {
                        self.emitter.emit_error(Xml5Error::UnexpectedSymbolOrEof(next_char));
                        reconsume!(BogusComment);
                    }
                    Some(x) => {
                        self.emitter.create_pi_tag(x);
                        switch_to!(PiTarget);
                    }
                }
            }
            PiTarget => {
                match self.read_fast_until(&[b'\t', b'\n', b' ']) {
                    Char(b'\t') | Char(b'\n') | Char(b' ') => switch_to!(PiTargetAfter),
                    Char(b'?') => switch_to!(PiAfter),
                    InterNeedle(x) => self.emitter.append_pi_target(x),
                    _ => {
                        self.emitter.emit_pi();
                        self.emitter.emit_error(Xml5Error::UnexpectedEof);
                        reconsume!(Data);
                    }
                }
            }
            PiTargetAfter => {
                match next_char {
                    Some(b'\t') | Some(b'\n') | Some(b' ') => {}
                    _ => reconsume!(PiData),
                }
            }
            PiData => {
                match self.read_fast_until(&[b'?']) {
                    Char(b'?') => switch_to!(PiAfter),
                    InterNeedle(x) => self.emitter.append_pi_data(x),
                    _ => {
                        self.emitter.emit_error(Xml5Error::UnexpectedEof);
                        self.emitter.emit_pi();
                        reconsume!(Data);
                    }
                }
            }
            PiAfter => {
                match next_char {
                    Some(b'>') => {
                        self.emitter.emit_pi();
                        switch_to!(Data);
                    }
                    Some(b'?') => self.emitter.append_pi_data_byte(b'?'),
                    _ => reconsume!(PiData),
                }
            }
            MarkupDecl => {
                if self.try_read_slice_exact("--") {
                    self.emitter.create_comment_token();
                    switch_to!(CommentStart)
                } else if self.try_read_slice("DOCTYPE", false) {
                    switch_to!(Doctype)
                } else if self.try_read_slice_exact("[CDATA[") {
                    switch_to!(Cdata)
                } else {
                    self.emitter.emit_error(Xml5Error::IncorrectlyOpenedComment);
                    switch_to!(BogusComment)
                }
            }
            CommentStart => {
                match next_char {
                    Some(b'-') => switch_to!(CommentStartDash),
                    Some(b'>') => {
                        self.emitter.emit_error(Xml5Error::AbruptClosingEmptyComment);
                        switch_to!(Data);
                        self.emitter.emit_comment();
                    }
                    _ => reconsume!(Comment),
                }
            }
            CommentStartDash => {
                match next_char {
                    Some(b'-') => switch_to!(CommentEnd),
                    Some(b'>') => {
                        self.emitter.emit_error(Xml5Error::AbruptClosingEmptyComment);
                        switch_to!(Data);
                        self.emitter.emit_comment();
                    }
                    None => {
                        self.emitter.emit_error(Xml5Error::EofInComment);
                        self.emitter.emit_comment();
                        self.emitter.emit_eof();
                    }
                    _ => {
                        self.emitter.append_to_comment_data(b'-');
                        reconsume!(Comment);
                    }
                }
            }
            Comment => {
                match self.read_fast_until(&[b'<', b'-']) {
                    InterNeedle(buf) => {
                        self.emitter.append_to_comment(buf);
                    }
                    Char(b'<') => {
                        self.emitter.append_to_comment_data(b'<');
                        switch_to!(CommentLessThan)
                    }
                    Char(b'-') => switch_to!(CommentEndDash),
                    _ => {
                        self.emitter.emit_error(Xml5Error::EofInComment);
                        self.emitter.emit_comment();
                        self.emitter.emit_eof();
                    }
                }
            }
            CommentLessThan => {
                match next_char {
                    Some(b'!') => {
                        self.emitter.append_to_comment_data(b'!');
                        switch_to!(CommentLessThanBang);
                    }
                    Some(b'<') => {
                        self.emitter.append_to_comment_data(b'<');
                    }
                    _ => reconsume!(Comment),
                }
            }
            CommentLessThanBang => {
                match next_char {
                    Some(b'-') => switch_to!(CommentLessThanBangDash),
                    _ => reconsume!(Comment),
                }
            }
            CommentLessThanBangDash => {
                match next_char {
                    Some(b'-') => switch_to!(CommentLessThanBangDashDash),
                    _ => reconsume!(CommentEndDash),
                }
            }
            CommentLessThanBangDashDash => {
                match next_char {
                    Some(b'>') | None => switch_to!(CommentEnd),
                    _ => reconsume!(CommentEndDash),
                }
            }
            CommentEndDash => {
                match next_char {
                    Some(b'-') => switch_to!(CommentEnd),
                    None => {
                        self.emitter.emit_error(Xml5Error::EofInComment);
                        self.emitter.emit_comment();
                        self.emitter.emit_eof();
                    }
                    _ => {
                        self.emitter.append_to_comment_data(b'-');
                        reconsume!(Comment);
                    }
                }
            }
            CommentEnd => {
                match next_char {
                    Some(b'>') => {
                        switch_to!(Data);
                        self.emitter.emit_comment();
                    }
                    Some(b'!') => switch_to!(CommentEndBang),
                    Some(b'-') => self.emitter.append_to_comment_data(b'-'),
                    None => {
                        self.emitter.emit_error(Xml5Error::EofInComment);
                        self.emitter.emit_comment();
                        self.emitter.emit_eof();
                    }
                    _ => {
                        self.emitter.append_to_comment("--");
                        reconsume!(Comment);
                    }
                }
            }
            CommentEndBang => {
                match next_char {
                    Some(b'-') => {
                        self.emitter.append_to_comment("-!");
                        switch_to!(CommentEndDash);
                    }
                    Some(b'>') => {
                        self.emitter.emit_error(Xml5Error::GreaterThanInComment);
                        switch_to!(Data);
                        self.emitter.emit_comment();
                    }
                    None => {
                        self.emitter.emit_error(Xml5Error::EofInComment);
                        self.emitter.emit_comment();
                        self.emitter.emit_eof();
                    }
                    _ => {
                        self.emitter.append_to_comment("--");
                        reconsume!(Comment)
                    }
                }
            }
            Cdata => {
                match self.read_fast_until(&[b']']) {
                    Char(b']') => switch_to!(CdataBracket),
                    InterNeedle(buf) => self.emitter.emit_chars(buf),
                    EOF | _ => {
                        self.emitter.emit_error(Xml5Error::EofInCdata);
                        reconsume!(Data);
                    }
                }
            }
            CdataBracket => {
                match next_char {
                    Some(b']') => switch_to!(CdataEnd),
                    None => {
                        self.emitter.emit_error(Xml5Error::EofInCdata);
                        reconsume!(Data);
                    }
                    Some(chr) => {
                        self.emitter.emit_char(b']');
                        self.emitter.emit_char(chr);
                        switch_to!(CdataBracket);
                    }
                }
            }
            CdataEnd => {
                match next_char {
                    Some(b'>') => switch_to!(Data),
                    Some(b']') => self.emitter.emit_char(b']'),
                    None => {
                        self.emitter.emit_error(Xml5Error::EofInCdata);
                        reconsume!(Data);
                    }
                    Some(chr) => {
                        self.emitter.emit_chars("]]");
                        self.emitter.emit_char(chr);
                        switch_to!(Cdata);
                    }
                }
            }
            BogusComment => {
                match self.read_fast_until(&[b'>']) {
                    Char(_) => {
                        switch_to!(Data);
                    }
                    InterNeedle(buf) => self.emitter.append_to_comment(buf),
                    _ => {
                        self.emitter.emit_comment();
                        self.emitter.emit_eof();
                    }
                }
            }
            Doctype => {
                match next_char {
                    Some(b'\t') | Some(b'\n') | Some(b' ') => switch_to!(BeforeDoctypeName),
                    None => {
                        self.emitter.emit_error(Xml5Error::EofInDoctype);
                        self.emitter.emit_doctype();
                        self.emitter.emit_eof();
                    }
                    _ => {
                        self.emitter.emit_error(Xml5Error::MissingWhitespaceDoctype);
                        reconsume!(BeforeDoctypeName);
                    }
                }
            }
            BeforeDoctypeName => {
                match next_char {
                    Some(b'\t') | Some(b'\n') | Some(b' ') => (),
                    Some(b'>') => {
                        self.emitter.emit_error(Xml5Error::MissingDoctypeName);
                        self.emitter.emit_doctype();
                        switch_to!(Data);
                    }
                    None => {
                        self.emitter.emit_error(Xml5Error::EofInDoctype);
                        self.emitter.emit_doctype();
                        self.emitter.emit_eof();
                    }
                    Some(x) => {
                        self.emitter.create_doctype();
                        let chr = if x.is_ascii_uppercase() {
                            x.to_ascii_lowercase()
                        } else {
                            x
                        };
                        self.emitter.append_doctype_name(chr);
                        switch_to!(DoctypeName);
                    }
                }
            }
            DoctypeName => {
                match next_char {
                    Some(b'\t') | Some(b'\n') | Some(b' ') => switch_to!(AfterDoctypeName),
                    Some(b'>') => {
                        self.emitter.emit_doctype();
                        switch_to!(Data);
                    }
                    None => {
                        self.emitter.emit_error(Xml5Error::EofInDoctype);
                        self.emitter.emit_doctype();
                        self.emitter.emit_eof();
                    }
                    Some(x) => {
                        let chr = if x.is_ascii_uppercase() {
                            x.to_ascii_lowercase()
                        } else {
                            x
                        };
                        self.emitter.append_doctype_name(chr);
                        switch_to!(DoctypeName);
                    }
                }
            }
            AfterDoctypeName => {
                match next_char {
                    Some(b'\t') | Some(b'\n') | Some(b' ') => (),
                    Some(b'>') => {
                        switch_to!(Data);
                        self.emitter.emit_doctype();
                    }
                    None => {
                        self.emitter.emit_error(Xml5Error::EofInDoctype);
                        self.emitter.emit_doctype();
                        self.emitter.emit_eof();
                    }
                    _ => {
                        if self.try_read_slice("PUBLIC", false) {
                            switch_to!(AfterDoctypeKeyword(Public))
                        } else if self.try_read_slice("SYSTEM", false) {
                            switch_to!(AfterDoctypeKeyword(System))
                        } else {
                            self.emitter.emit_error(Xml5Error::InvalidCharactersInAfterDoctypeName);
                            switch_to!(BogusComment);
                        }
                    }
                }
            },
            AfterDoctypeKeyword(Public) => {
                match next_char {
                    Some(b'\t') | Some(b'\n') | Some(b' ') => switch_to!(BeforeDoctypeIdentifier(Public)),
                    Some(b'"') => {
                        self.emitter.emit_error(Xml5Error::MissingWhitespaceAfterDoctypeKeyword);
                        self.emitter.clear_doctype_id(Public);
                        switch_to!(DoctypeIdentifierDoubleQuoted(Public));
                    },
                    Some(b'\'') => {
                        self.emitter.emit_error(Xml5Error::MissingWhitespaceAfterDoctypeKeyword);
                        self.emitter.clear_doctype_id(Public);
                        switch_to!(DoctypeIdentifierSingleQuoted(Public));
                    }
                    Some(b'>') => {
                        self.emitter.emit_error(Xml5Error::MissingWhitespaceAfterDoctypeKeyword);
                        self.emitter.clear_doctype_id(Public);
                        switch_to!(Data);
                    }
                    None => {
                        self.emitter.emit_error(Xml5Error::EofInDoctype);
                        self.emitter.emit_doctype();
                        self.emitter.emit_eof();
                    }
                    _ => {
                        self.emitter.emit_error(Xml5Error::MissingQuoteBeforeIdentifier);
                        reconsume!(BogusDoctype);
                    }
                }
            }
            AfterDoctypeKeyword(System) => {
                match next_char {
                    Some(b'\t') | Some(b'\n') | Some(b' ') => switch_to!(BeforeDoctypeIdentifier(System)),
                    Some(b'"') => {
                        self.emitter.emit_error(Xml5Error::MissingWhitespaceAfterDoctypeKeyword);
                        self.emitter.clear_doctype_id(System);
                        switch_to!(DoctypeIdentifierDoubleQuoted(System));
                    },
                    Some(b'\'') => {
                        self.emitter.emit_error(Xml5Error::MissingWhitespaceAfterDoctypeKeyword);
                        self.emitter.clear_doctype_id(System);
                        switch_to!(DoctypeIdentifierSingleQuoted(System));
                    }
                    Some(b'>') => {
                        self.emitter.emit_error(Xml5Error::MissingWhitespaceAfterDoctypeKeyword);
                        self.emitter.clear_doctype_id(System);
                        switch_to!(Data);
                    }
                    None => {
                        self.emitter.emit_error(Xml5Error::EofInDoctype);
                        self.emitter.emit_doctype();
                        self.emitter.emit_eof();
                    }
                    _ => {
                        self.emitter.emit_error(Xml5Error::MissingQuoteBeforeIdentifier);
                        reconsume!(BogusDoctype);
                    }
                }
            }
            BeforeDoctypeIdentifier(kind) => {
                match next_char {
                    Some(b'\t') | Some(b'\n') | Some(b' ') => (),
                    Some(b'"') => {
                        self.emitter.clear_doctype_id(kind);
                        switch_to!(DoctypeIdentifierDoubleQuoted(kind));
                    },
                    Some(b'\'') => {
                        self.emitter.clear_doctype_id(kind);
                        switch_to!(DoctypeIdentifierSingleQuoted(kind));
                    }
                    Some(b'>') => {
                        self.emitter.emit_error(Xml5Error::MissingDoctypeIdentifier);
                        switch_to!(Data);
                        self.emitter.emit_doctype();
                    }
                    None => {
                        self.emitter.emit_error(Xml5Error::EofInDoctype);
                        self.emitter.emit_doctype();
                        self.emitter.emit_eof();
                    }
                    _ => {
                        self.emitter.emit_error(Xml5Error::MissingQuoteBeforeIdentifier);
                        reconsume!(BogusDoctype);
                    }
                }
            }
            DoctypeIdentifierDoubleQuoted(kind) => {
                match self.read_fast_until(&[b'"', b'>']) {
                    Char(b'"') => switch_to!(AfterDoctypeIdentifier(kind)),
                    Char(b'>') => {
                        self.emitter.emit_error(Xml5Error::AbruptEndDoctypeIdentifier);
                        switch_to!(Data);
                        self.emitter.emit_doctype();
                    }
                    InterNeedle(buf) => {
                        self.emitter.append_doctype_id(buf);
                    }
                    _ => {
                        self.emitter.emit_error(Xml5Error::EofInDoctype);
                        self.emitter.emit_doctype();
                        self.emitter.emit_eof();
                    }
                }
            }
            DoctypeIdentifierSingleQuoted(kind) => {
                match self.read_fast_until(&[b'\'', b'>']) {
                    Char(b'\'') => switch_to!(AfterDoctypeIdentifier(kind)),
                    Char(b'>') => {
                        self.emitter.emit_error(Xml5Error::AbruptEndDoctypeIdentifier);
                        switch_to!(Data);
                        self.emitter.emit_doctype();
                    }
                    InterNeedle(buf) => {
                        self.emitter.append_doctype_id(buf);
                    }
                    _ => {
                        self.emitter.emit_error(Xml5Error::EofInDoctype);
                        self.emitter.emit_doctype();
                        self.emitter.emit_eof();
                    }
                }
            }
            AfterDoctypeIdentifier(Public) => {
                match next_char {
                    Some(b'\t') | Some(b'\n') | Some(b' ') => switch_to!(BetweenDoctypePublicAndSystemIdentifiers),
                    Some(b'>') => {
                        switch_to!(Data);
                        self.emitter.emit_doctype();
                    }
                    Some(b'"') => {
                        self.emitter.emit_error(Xml5Error::MissingWhitespaceBetweenDoctypePublicAndSystem);
                        switch_to!(DoctypeIdentifierDoubleQuoted(System));
                    }
                    Some(b'\'') => {
                        self.emitter.emit_error(Xml5Error::MissingWhitespaceBetweenDoctypePublicAndSystem);
                        switch_to!(DoctypeIdentifierSingleQuoted(System));
                    }
                    None => {
                        self.emitter.emit_error(Xml5Error::EofInDoctype);
                        self.emitter.emit_doctype();
                        self.emitter.emit_eof();
                    }
                    _ => {
                        self.emitter.emit_error(Xml5Error::MissingQuoteBeforeIdentifier);
                        reconsume!(BogusDoctype);
                    }
                }
            }
            BetweenDoctypePublicAndSystemIdentifiers => {
                match next_char {
                    Some(b'\t') | Some(b'\n') | Some(b' ') => (),
                    Some(b'"') => {
                        self.emitter.clear_doctype_id(System);
                        switch_to!(DoctypeIdentifierDoubleQuoted(System))
                    }
                    Some(b'\'') => {
                        self.emitter.clear_doctype_id(System);
                        switch_to!(DoctypeIdentifierSingleQuoted(System))
                    }
                    Some(b'>') => {
                        switch_to!(Data);
                        self.emitter.emit_doctype();
                    }
                    None => {
                        self.emitter.emit_error(Xml5Error::EofInDoctype);
                        self.emitter.emit_doctype();
                        self.emitter.emit_eof();
                    }
                    _ => {
                        self.emitter.emit_error(Xml5Error::MissingQuoteBeforeIdentifier);
                        reconsume!(BogusDoctype);
                    }
                }
            }
            AfterDoctypeIdentifier(System) => {
                match next_char {
                    Some(b'\t') | Some(b'\n') | Some(b' ') => (),
                    Some(b'>') => {
                        self.emitter.emit_doctype();
                        switch_to!(Data);
                    }
                    _ => {
                        self.emitter.emit_error(Xml5Error::MissingQuoteBeforeIdentifier);
                        reconsume!(BogusDoctype);
                    }
                }
            }
            BogusDoctype => {
                match next_char {
                    Some(b'>') => {
                        switch_to!(Data);
                        self.emitter.emit_doctype();
                    }
                    None => {
                        self.emitter.emit_doctype();
                        self.emitter.emit_eof();
                    }
                    _ => (),
                }
            }
            CharRefInData(_) => {
                // TODO char ref
            }
        };
        self.source.consume(amt);
        Control::Continue
    }
}

#[test]
fn test_read_until() {
    let source = "TestString";
    let mut buf = vec![];
    let mut xml = Tokenizer::from_str(source, &mut buf);

    assert_eq!(InterNeedle("TestSt".into()), xml.read_fast_until(&[b'r']));
    assert_eq!(Char(b'r'), xml.read_fast_until(&[b'r']));
    assert_eq!(InterNeedle("in".into()), xml.read_fast_until(&[b'g']));
    assert_eq!(Char(b'g'), xml.read_fast_until(&[b'g']));
    assert_eq!(EOF, xml.read_fast_until(&[b'r']));
}

#[test]
fn test_read_until2() {
    let source = "xyz_abc";
    let mut buf = vec![];
    let mut xml = Tokenizer::from_str(source, &mut buf);

    assert_eq!(Char(b'x'), xml.read_fast_until(&[b'x']));
    assert_eq!(Char(b'y'), xml.read_fast_until(&[b'y']));
    assert_eq!(Char(b'z'), xml.read_fast_until(&[b'z']));
}

#[test]
fn test_try_read_slice1() {
    let source = "xyz_abc";
    let mut buf = vec![];
    let mut xml = Tokenizer::from_str(source, &mut buf);

    assert!(!xml.try_read_slice("?A?", true));
    assert!(xml.try_read_slice("xyz", true));
    assert!(!xml.try_read_slice("?A?", true));
    assert!(xml.try_read_slice("_AbC", false));
    assert!(!xml.try_read_slice("_AbC", false));
}

