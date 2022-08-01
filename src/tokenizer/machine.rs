use std::io;
use std::io::BufRead;
use std::str::from_utf8;

use FastRead::EOF;

use crate::{Token, Tokenizer};
use crate::errors::{Xml5Error, Xml5Result};
use crate::errors::Xml5Error::{UnexpectedEof, UnexpectedSymbol, UnexpectedSymbolOrEof};
use crate::Token::Error;
use crate::tokenizer::{Control, TokenState};
use crate::tokenizer::Control::Eof;
use crate::tokenizer::emitter::{DefaultEmitter, Emitter};
use crate::tokenizer::reader::{BufferedInput, fast_find, FastRead};
use crate::tokenizer::reader::FastRead::{Char, InterNeedle};
use crate::tokenizer::TokenState::{*};

#[inline(always)]
pub(crate) fn is_(b: u8) -> bool {
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
            previous_needle: None,
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
            ($state:expr) => { {
                amt= 0; self.state = $state}
            };
        }



        match self.state {
            Data => {
                match self.read_fast_until(&[b'<', b'&']) {
                    Char(b'&') => switch_to!(CharRefInData),
                    Char(b'<') => switch_to!( Tag),
                    InterNeedle(text) => self.emitter.emit_chars(text),
                    _ => self.emitter.emit_eof(),
                }
            }
            // Todo char ref
            Tag => {
                match next_char {
                    Some(b'/') => switch_to!(EndTag),
                    Some(b'?') => switch_to!(Pi),
                    Some(b'!') => switch_to!(MarkupDecl),
                    None | Some(b'\t') | Some(b'\r') | Some(b'\n')
                    | Some(b' ') | Some(b':') | Some(b'<') | Some(b'>') => {
                        self.emitter.emit_error(UnexpectedSymbolOrEof(next_char));
                        self.emitter.emit_char('<');
                        switch_to!(Data);
                    }
                    Some(c) => {
                        self.emitter.create_tag(c);
                        switch_to!(TagName);
                    }
                }
            }
            TagName => {
                match self.read_fast_until(&[b'\t', b'\n', b' ', b'>', b'/']) {
                    Char(b'\t') | Char(b'\n') | Char(b' ') => switch_to!(TagAttrNameBefore),
                    Char(b'>') => {
                        self.emitter.emit_start_tag_token();
                        switch_to!(Data);
                    }
                    _ => {
                        self.emitter.emit_error(UnexpectedEof);
                        self.emitter.emit_token();
                        reconsume!(Data);
                    }
                    Char(b'/') => {
                        self.emitter.set_empty_tag();
                        switch_to!(TagEmpty);
                    }
                }
            }
            EndTag => {
                match next_char {
                    Some(b'>') => {
                        self.emitter.emit_short_end_tag();
                        switch_to!(Data);
                    }
                    None | Some(b'\t') | Some(b'\r') | Some(b'\n')
                    | Some(b' ') | Some(b':') | Some(b'<') => {
                        self.emitter.emit_error(UnexpectedSymbolOrEof(next_char));
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
                match self.read_fast_until(&[b'\t', b'\r', b'\n', b' ', b'/', b'>']) {
                    Char(b'\t') | Char(b'\r') | Char(b'\n') | Char(b' ') => {
                        switch_to!(EndTagNameAfter);
                    }
                    Char(b'/') => {
                        self.emitter.emit_error(UnexpectedSymbol('/'));
                        switch_to!(EndTagNameAfter);
                    }
                    Char(b'>') => {
                        self.emitter.emit_char('>');
                        switch_to!(Data);
                    }
                    InterNeedle(x) => self.emitter.append_tag(x),
                    _ => {
                        self.emitter.emit_error(UnexpectedEof);
                    }
                }
            }
            EndTagNameAfter => {
                match next_char {
                    Some(b'>') => {
                        self.emitter.emit_token();
                        switch_to!(Data);
                    }
                    Some(b' ') | Some(b'\r') | Some(b'\n') | Some(b'\t') => {}
                    None => {
                        self.emitter.emit_error(UnexpectedSymbolOrEof(None));
                        reconsume!(Data);
                    }
                    Some(x) => {
                        self.emitter.emit_error(UnexpectedSymbol(x as char));
                    }
                }
            }
            Pi => {
                match next_char {
                    None | Some(b' ') | Some(b'\r') | Some(b'\n') | Some(b'\t') => {
                        self.emitter.emit_error(UnexpectedSymbolOrEof(next_char));
                        reconsume!(BogusComment);
                    }
                    Some(x) => {
                        self.emitter.create_pi_tag(x);
                        switch_to!(PiTarget);
                    }
                }
            }
            PiTarget => {
                match self.read_fast_until(&[b'\t', b'\r', b'\n', b' ']) {
                    Char(b'\t') | Char(b'\r') | Char(b'\n') | Char(b' ') => switch_to!(PiTargetAfter),
                    Char(b'?') => switch_to!(PiAfter),
                    InterNeedle(x) => self.emitter.append_pi_target(x),
                    _ => {
                        self.emitter.emit_pi();
                        self.emitter.emit_error(UnexpectedEof);
                        reconsume!(Data);
                    }
                }
            }
            PiTargetAfter => {
                match next_char {
                    Some(b'\t') | Some(b'\n') | Some(b'\r') | Some(b' ') => {}
                    _ => { reconsume!(PiData); }
                }
            }
            PiData => {
                match self.read_fast_until(&[b'?']) {
                    Char(b'?') => switch_to!(PiAfter),
                    InterNeedle(x) => self.emitter.append_pi_data(x),
                    _ => {
                        self.emitter.emit_error(UnexpectedEof);
                        self.emitter.emit_pi();
                        reconsume!(Data);
                    }
                }
            }
            PiAfter => {
                match next_char {
                    Some(b'>') => {
                        self.emitter.emit_token();
                        switch_to!(Data);
                    }
                    Some(b'?') => self.emitter.append_pi_data_byte(b'?'),
                    _ => { reconsume!(PiData); }
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
                    _ => {
                        reconsume!(Comment);
                    }
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
                    EOF => {
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
                    _ => { reconsume!(Comment); }
                }
            }
            CommentLessThanBang => {
                match next_char {
                    Some(b'-') => switch_to!(CommentLessThanBangDash),
                    _ => { reconsume!(Comment); }
                }
            }
            CommentLessThanBangDash => {
                match next_char {
                    Some(b'-') => switch_to!(CommentLessThanBangDashDash),
                    _ =>  {reconsume!(CommentEndDash);}
                }
            }
            CommentLessThanBangDashDash => {
                match next_char {
                    Some(b'>') => switch_to!(CommentEnd),
                    _ => { reconsume!(CommentEndDash); }
                }
            }
            _ => {}
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

