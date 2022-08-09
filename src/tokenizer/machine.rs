use std::io::BufRead;

use FastRead::EOF;

use crate::errors::Xml5Error;
use crate::tokenizer::emitter::Emitter;
use crate::tokenizer::reader::FastRead::{Char, InterNeedle};
use crate::tokenizer::reader::{BufferedInput, FastRead};
use crate::tokenizer::AttrValueKind::{DoubleQuoted, SingleQuoted, Unquoted};
use crate::tokenizer::Control;
use crate::tokenizer::Control::Eof;
use crate::tokenizer::DoctypeKind::{Public, System};
use crate::tokenizer::TokenState::*;
use crate::Tokenizer;

impl<'a, S: BufRead> Tokenizer<'a, S> {
    #[inline]
    pub(crate) fn next_state(&mut self) -> Control {
        let mut amt = 1;
        let next_char = match self.source.peek_byte() {
            Ok(None) => {
                self.eof = true;
                return Eof;
            }
            Ok(x) => x,
            Err(e) => return Control::Err(e),
        };

        macro_rules! switch_to {
            ($state:expr) => {
                self.state = $state
            };
        }

        macro_rules! reconsume {
            ($state:expr) => {{
                amt = 0;
                self.state = $state;
            }};
        }

        match self.state {
            Data => match self.read_fast_until(&[b'<', b'&']) {
                Char(b'&') => switch_to!(CharRefInData(Unquoted)),
                Char(b'<') => switch_to!(TagOpen),
                InterNeedle(start, len) => self.emit_chars((start, len)),
                _ => self.emit_eof(),
            },
            TagOpen => match next_char {
                Some(b'/') => switch_to!(EndTagOpen),
                Some(b'?') => switch_to!(Pi),
                Some(b'!') => switch_to!(MarkupDecl),
                None | Some(b'\t') | Some(b'\n') | Some(b' ') | Some(b':') | Some(b'<')
                | Some(b'>') => {
                    self.emit_error(Xml5Error::UnexpectedSymbolOrEof(next_char));
                    self.emit_chars(b'<');
                    switch_to!(Data);
                }
                Some(c) => {
                    self.create_tag();
                    self.append_tag(c);
                    switch_to!(TagName);
                }
            },
            EndTagOpen => match next_char {
                Some(b'>') => {
                    self.emit_tag();
                    switch_to!(Data);
                }
                None | Some(b'\t') | Some(b'\n') | Some(b' ') | Some(b':') | Some(b'<') => {
                    self.emit_error(Xml5Error::UnexpectedSymbolOrEof(next_char));
                    self.emit_chars("</");
                    reconsume!(Data);
                }
                Some(c) => {
                    self.create_end_tag();
                    self.append_tag(c);
                    switch_to!(EndTagName);
                }
            },
            EndTagName => match self.read_fast_until(&[b'\t', b'\n', b' ', b'/', b'>']) {
                Char(b'\t') | Char(b'\n') | Char(b' ') => {
                    switch_to!(EndTagNameAfter);
                }
                Char(b'/') => {
                    self.emit_error(Xml5Error::UnexpectedSymbol('/'));
                    switch_to!(EndTagNameAfter);
                }
                Char(b'>') => {
                    self.append_tag(b'>');
                    switch_to!(Data);
                }
                InterNeedle(start, len) => self.append_tag((start, len)),
                _ => {
                    self.emit_error(Xml5Error::UnexpectedEof);
                }
            },
            EndTagNameAfter => match next_char {
                Some(b'>') => {
                    self.emit_tag();
                    switch_to!(Data);
                }
                Some(b' ') | Some(b'\n') | Some(b'\t') => {}
                None => {
                    self.emit_error(Xml5Error::UnexpectedSymbolOrEof(None));
                    reconsume!(Data);
                }
                Some(x) => {
                    self.emit_error(Xml5Error::UnexpectedSymbol(x as char));
                }
            },
            TagName => match self.read_fast_until(&[b'\t', b'\n', b' ', b'>', b'/']) {
                Char(b'\t') | Char(b'\n') | Char(b' ') => switch_to!(TagAttrNameBefore),
                Char(b'>') => {
                    self.emit_tag();
                    switch_to!(Data);
                }
                Char(b'/') => {
                    self.set_empty_tag();
                    switch_to!(EmptyTag);
                }
                InterNeedle(start, len) => {
                    self.append_tag((start, len));
                }
                _ => {
                    self.emit_error(Xml5Error::EofInTag);
                    self.emit_tag();
                    reconsume!(Data);
                }
            },
            EmptyTag => match next_char {
                Some(b'>') => {
                    self.emit_tag();
                    switch_to!(Data);
                }
                _ => reconsume!(TagAttrValueBefore),
            },
            TagAttrNameBefore => match next_char {
                Some(b't') | Some(b't') | Some(b't') => (),
                Some(b'>') => {
                    self.emit_tag();
                    switch_to!(Data);
                }
                Some(b'/') => {
                    self.set_empty_tag();
                    switch_to!(EmptyTag);
                }
                Some(b':') => self.emit_error(Xml5Error::ColonBeforeAttrName),
                None => {
                    self.emit_error(Xml5Error::EofInTag);
                    self.emit_tag();
                    reconsume!(Data);
                }
                Some(c) => {
                    self.create_attr();
                    self.push_attr_values(c);
                    switch_to!(TagAttrName);
                }
            },
            TagAttrName => match self.read_fast_until(&[b'\t', b'\n', b' ', b'=', b'>', b'/']) {
                Char(b'\t') | Char(b'\n') | Char(b' ') => switch_to!(TagAttrValueBefore),
                Char(b'=') => switch_to!(TagAttrValueBefore),
                Char(b'>') => {
                    self.emit_tag();
                    switch_to!(Data);
                }
                Char(b'/') => {
                    self.set_empty_tag();
                    switch_to!(EmptyTag);
                }
                EOF | _ => {
                    self.emit_error(Xml5Error::EofInTag);
                    self.emit_tag();
                    reconsume!(Data);
                }
            },
            TagAttrNameAfter => match next_char {
                Some(b'\t') | Some(b'\n') | Some(b' ') => (),
                Some(b'=') => switch_to!(TagAttrValueBefore),
                Some(b'>') => {
                    self.emit_tag();
                    switch_to!(EmptyTag);
                }
                Some(b'/') => {
                    self.set_empty_tag();
                    switch_to!(EmptyTag);
                }
                None => {
                    self.emit_error(Xml5Error::EofInTag);
                    self.emit_tag();
                    reconsume!(Data);
                }
                Some(c) => {
                    self.create_attr();
                    self.push_attr_name(c);
                    switch_to!(TagAttrName)
                }
            },
            TagAttrValueBefore => match next_char {
                Some(b'\t') | Some(b'\n') | Some(b' ') => (),
                Some(b'"') => switch_to!(TagAttrValue(DoubleQuoted)),
                Some(b'\'') => switch_to!(TagAttrValue(SingleQuoted)),
                Some(b'&') => reconsume!(TagAttrValue(Unquoted)),
                Some(b'>') => {
                    self.emit_tag();
                    switch_to!(Data);
                }
                None => {
                    self.emit_error(Xml5Error::EofInTag);
                    self.emit_tag();
                    reconsume!(Data);
                }
                Some(c) => {
                    self.push_attr_values(c);
                    switch_to!(TagAttrValue(Unquoted));
                }
            },
            TagAttrValue(DoubleQuoted) => match self.read_fast_until(&[b'&', b'"']) {
                Char(b'"') => switch_to!(TagAttrNameBefore),
                Char(_) => switch_to!(CharRefInData(DoubleQuoted)),
                InterNeedle(start, len) => self.push_attr_values((start, len)),
                EOF => {
                    self.emit_error(Xml5Error::EofInTag);
                    self.emit_tag();
                    reconsume!(Data);
                }
            },
            TagAttrValue(SingleQuoted) => match self.read_fast_until(&[b'&', b'\'']) {
                Char(b'\'') => switch_to!(TagAttrNameBefore),
                Char(_) => switch_to!(CharRefInData(DoubleQuoted)),
                InterNeedle(start, len) => self.push_attr_values((start, len)),
                EOF => {
                    self.emit_error(Xml5Error::EofInTag);
                    self.emit_tag();
                    reconsume!(Data);
                }
            },
            TagAttrValue(Unquoted) => {
                match self.read_fast_until(&[b'\t', b'\n', b' ', b'&', b'>']) {
                    Char(b'\t') | Char(b'\n') | Char(b' ') => switch_to!(TagAttrNameBefore),
                    Char(b'&') => switch_to!(CharRefInData(Unquoted)),
                    Char(_) => {
                        self.emit_tag();
                        switch_to!(Data);
                    }
                    InterNeedle(start, len) => self.push_attr_values((start, len)),
                    EOF => {
                        self.emit_error(Xml5Error::EofInTag);
                        self.emit_tag();
                        reconsume!(Data);
                    }
                }
            }
            Pi => match next_char {
                None | Some(b' ') | Some(b'\n') | Some(b'\t') => {
                    self.emit_error(Xml5Error::UnexpectedSymbolOrEof(next_char));
                    reconsume!(BogusComment);
                }
                Some(c) => {
                    self.create_pi_tag();
                    self.append_pi_data(c);
                    switch_to!(PiTarget);
                }
            },
            PiTarget => match self.read_fast_until(&[b'\t', b'\n', b' ']) {
                Char(b'\t') | Char(b'\n') | Char(b' ') => switch_to!(PiTargetAfter),
                Char(b'?') => switch_to!(PiAfter),
                InterNeedle(start, len) => self.append_pi_target((start, len)),
                _ => {
                    self.emit_pi();
                    self.emit_error(Xml5Error::UnexpectedEof);
                    reconsume!(Data);
                }
            },
            PiTargetAfter => match next_char {
                Some(b'\t') | Some(b'\n') | Some(b' ') => {}
                _ => reconsume!(PiData),
            },
            PiData => match self.read_fast_until(&[b'?']) {
                Char(b'?') => switch_to!(PiAfter),
                InterNeedle(start, end) => self.append_pi_data((start, end)),
                _ => {
                    self.emit_error(Xml5Error::UnexpectedEof);
                    self.emit_pi();
                    reconsume!(Data);
                }
            },
            PiAfter => match next_char {
                Some(b'>') => {
                    self.emit_pi();
                    switch_to!(Data);
                }
                Some(b'?') => self.append_pi_data(b'?'),
                _ => reconsume!(PiData),
            },
            MarkupDecl => {
                if self.try_read_slice_exact("--") {
                    self.create_comment_token();
                    switch_to!(CommentStart)
                } else if self.try_read_slice("DOCTYPE", false) {
                    switch_to!(Doctype)
                } else if self.try_read_slice_exact("[CDATA[") {
                    switch_to!(Cdata)
                } else {
                    self.emit_error(Xml5Error::IncorrectlyOpenedComment);
                    switch_to!(BogusComment)
                }
            }
            CommentStart => match next_char {
                Some(b'-') => switch_to!(CommentStartDash),
                Some(b'>') => {
                    self.emit_error(Xml5Error::AbruptClosingEmptyComment);
                    switch_to!(Data);
                    self.emit_comment();
                }
                _ => reconsume!(Comment),
            },
            CommentStartDash => match next_char {
                Some(b'-') => switch_to!(CommentEnd),
                Some(b'>') => {
                    self.emit_error(Xml5Error::AbruptClosingEmptyComment);
                    switch_to!(Data);
                    self.emit_comment();
                }
                None => {
                    self.emit_error(Xml5Error::EofInComment);
                    self.emit_comment();
                    self.emit_eof();
                }
                _ => {
                    self.append_to_comment(b'-');
                    reconsume!(Comment);
                }
            },
            Comment => match self.read_fast_until(&[b'<', b'-']) {
                InterNeedle(start, end) => {
                    self.append_to_comment((start, end));
                }
                Char(b'<') => {
                    self.append_to_comment(b'<');
                    switch_to!(CommentLessThan)
                }
                Char(b'-') => switch_to!(CommentEndDash),
                _ => {
                    self.emit_error(Xml5Error::EofInComment);
                    self.emit_comment();
                    self.emit_eof();
                }
            },
            CommentLessThan => match next_char {
                Some(b'!') => {
                    self.append_to_comment(b'!');
                    switch_to!(CommentLessThanBang);
                }
                Some(b'<') => {
                    self.append_to_comment(b'<');
                }
                _ => reconsume!(Comment),
            },
            CommentLessThanBang => match next_char {
                Some(b'-') => switch_to!(CommentLessThanBangDash),
                _ => reconsume!(Comment),
            },
            CommentLessThanBangDash => match next_char {
                Some(b'-') => switch_to!(CommentLessThanBangDashDash),
                _ => reconsume!(CommentEndDash),
            },
            CommentLessThanBangDashDash => match next_char {
                Some(b'>') | None => switch_to!(CommentEnd),
                _ => reconsume!(CommentEndDash),
            },
            CommentEndDash => match next_char {
                Some(b'-') => switch_to!(CommentEnd),
                None => {
                    self.emit_error(Xml5Error::EofInComment);
                    self.emit_comment();
                    self.emit_eof();
                }
                _ => {
                    self.append_to_comment(b'-');
                    reconsume!(Comment);
                }
            },
            CommentEnd => match next_char {
                Some(b'>') => {
                    switch_to!(Data);
                    self.emit_comment();
                }
                Some(b'!') => switch_to!(CommentEndBang),
                Some(b'-') => self.append_to_comment(b'-'),
                None => {
                    self.emit_error(Xml5Error::EofInComment);
                    self.emit_comment();
                    self.emit_eof();
                }
                _ => {
                    self.append_to_comment("--");
                    reconsume!(Comment);
                }
            },
            CommentEndBang => match next_char {
                Some(b'-') => {
                    self.append_to_comment("-!");
                    switch_to!(CommentEndDash);
                }
                Some(b'>') => {
                    self.emit_error(Xml5Error::GreaterThanInComment);
                    switch_to!(Data);
                    self.emit_comment();
                }
                None => {
                    self.emit_error(Xml5Error::EofInComment);
                    self.emit_comment();
                    self.emit_eof();
                }
                _ => {
                    self.append_to_comment("--");
                    reconsume!(Comment)
                }
            },
            Cdata => match self.read_fast_until(&[b']']) {
                Char(b']') => switch_to!(CdataBracket),
                InterNeedle(start, end) => self.emit_chars((start, end)),
                EOF | _ => {
                    self.emit_error(Xml5Error::EofInCdata);
                    reconsume!(Data);
                }
            },
            CdataBracket => match next_char {
                Some(b']') => switch_to!(CdataEnd),
                None => {
                    self.emit_error(Xml5Error::EofInCdata);
                    reconsume!(Data);
                }
                Some(c) => {
                    self.emit_chars(b']');
                    self.emit_chars(c);
                    switch_to!(CdataBracket);
                }
            },
            CdataEnd => match next_char {
                Some(b'>') => switch_to!(Data),
                Some(b']') => self.emit_chars(b']'),
                None => {
                    self.emit_error(Xml5Error::EofInCdata);
                    reconsume!(Data);
                }
                Some(c) => {
                    self.emit_chars("]]");
                    self.emit_chars(c);
                    switch_to!(Cdata);
                }
            },
            BogusComment => match self.read_fast_until(&[b'>']) {
                Char(_) => {
                    switch_to!(Data);
                }
                InterNeedle(start, len) => self.append_to_comment((start, len)),
                _ => {
                    self.emit_comment();
                    self.emit_eof();
                }
            },
            Doctype => match next_char {
                Some(b'\t') | Some(b'\n') | Some(b' ') => switch_to!(BeforeDoctypeName),
                None => {
                    self.emit_error(Xml5Error::EofInDoctype);
                    self.emit_doctype();
                    self.emit_eof();
                }
                _ => {
                    self.emit_error(Xml5Error::MissingWhitespaceDoctype);
                    reconsume!(BeforeDoctypeName);
                }
            },
            BeforeDoctypeName => match next_char {
                Some(b'\t') | Some(b'\n') | Some(b' ') => (),
                Some(b'>') => {
                    self.emit_error(Xml5Error::MissingDoctypeName);
                    self.emit_doctype();
                    switch_to!(Data);
                }
                None => {
                    self.emit_error(Xml5Error::EofInDoctype);
                    self.emit_doctype();
                    self.emit_eof();
                }
                Some(x) => {
                    self.create_doctype();
                    let chr = if x.is_ascii_uppercase() {
                        x.to_ascii_lowercase()
                    } else {
                        x
                    };
                    self.append_doctype_name(chr);
                    switch_to!(DoctypeName);
                }
            },
            DoctypeName => match next_char {
                Some(b'\t') | Some(b'\n') | Some(b' ') => switch_to!(AfterDoctypeName),
                Some(b'>') => {
                    self.emit_doctype();
                    switch_to!(Data);
                }
                None => {
                    self.emit_error(Xml5Error::EofInDoctype);
                    self.emit_doctype();
                    self.emit_eof();
                }
                Some(x) => {
                    let chr = if x.is_ascii_uppercase() {
                        x.to_ascii_lowercase()
                    } else {
                        x
                    };
                    self.append_doctype_name(chr);
                    switch_to!(DoctypeName);
                }
            },
            AfterDoctypeName => match next_char {
                Some(b'\t') | Some(b'\n') | Some(b' ') => (),
                Some(b'>') => {
                    switch_to!(Data);
                    self.emit_doctype();
                }
                None => {
                    self.emit_error(Xml5Error::EofInDoctype);
                    self.emit_doctype();
                    self.emit_eof();
                }
                _ => {
                    if self.try_read_slice("PUBLIC", false) {
                        switch_to!(AfterDoctypeKeyword(Public))
                    } else if self.try_read_slice("SYSTEM", false) {
                        switch_to!(AfterDoctypeKeyword(System))
                    } else {
                        self.emit_error(Xml5Error::InvalidCharactersInAfterDoctypeName);
                        switch_to!(BogusComment);
                    }
                }
            },
            AfterDoctypeKeyword(Public) => match next_char {
                Some(b'\t') | Some(b'\n') | Some(b' ') => {
                    switch_to!(BeforeDoctypeIdentifier(Public))
                }
                Some(b'"') => {
                    self.emit_error(Xml5Error::MissingWhitespaceAfterDoctypeKeyword);
                    self.clear_doctype_id(Public);
                    switch_to!(DoctypeIdentifierDoubleQuoted(Public));
                }
                Some(b'\'') => {
                    self.emit_error(Xml5Error::MissingWhitespaceAfterDoctypeKeyword);
                    self.clear_doctype_id(Public);
                    switch_to!(DoctypeIdentifierSingleQuoted(Public));
                }
                Some(b'>') => {
                    self.emit_error(Xml5Error::MissingWhitespaceAfterDoctypeKeyword);
                    self.clear_doctype_id(Public);
                    switch_to!(Data);
                }
                None => {
                    self.emit_error(Xml5Error::EofInDoctype);
                    self.emit_doctype();
                    self.emit_eof();
                }
                _ => {
                    self.emit_error(Xml5Error::MissingQuoteBeforeIdentifier);
                    reconsume!(BogusDoctype);
                }
            },
            AfterDoctypeKeyword(System) => match next_char {
                Some(b'\t') | Some(b'\n') | Some(b' ') => {
                    switch_to!(BeforeDoctypeIdentifier(System))
                }
                Some(b'"') => {
                    self.emit_error(Xml5Error::MissingWhitespaceAfterDoctypeKeyword);
                    self.clear_doctype_id(System);
                    switch_to!(DoctypeIdentifierDoubleQuoted(System));
                }
                Some(b'\'') => {
                    self.emit_error(Xml5Error::MissingWhitespaceAfterDoctypeKeyword);
                    self.clear_doctype_id(System);
                    switch_to!(DoctypeIdentifierSingleQuoted(System));
                }
                Some(b'>') => {
                    self.emit_error(Xml5Error::MissingWhitespaceAfterDoctypeKeyword);
                    self.clear_doctype_id(System);
                    switch_to!(Data);
                }
                None => {
                    self.emit_error(Xml5Error::EofInDoctype);
                    self.emit_doctype();
                    self.emit_eof();
                }
                _ => {
                    self.emit_error(Xml5Error::MissingQuoteBeforeIdentifier);
                    reconsume!(BogusDoctype);
                }
            },
            BeforeDoctypeIdentifier(kind) => match next_char {
                Some(b'\t') | Some(b'\n') | Some(b' ') => (),
                Some(b'"') => {
                    self.clear_doctype_id(kind);
                    switch_to!(DoctypeIdentifierDoubleQuoted(kind));
                }
                Some(b'\'') => {
                    self.clear_doctype_id(kind);
                    switch_to!(DoctypeIdentifierSingleQuoted(kind));
                }
                Some(b'>') => {
                    self.emit_error(Xml5Error::MissingDoctypeIdentifier);
                    switch_to!(Data);
                    self.emit_doctype();
                }
                None => {
                    self.emit_error(Xml5Error::EofInDoctype);
                    self.emit_doctype();
                    self.emit_eof();
                }
                _ => {
                    self.emit_error(Xml5Error::MissingQuoteBeforeIdentifier);
                    reconsume!(BogusDoctype);
                }
            },
            DoctypeIdentifierDoubleQuoted(kind) => match self.read_fast_until(&[b'"', b'>']) {
                Char(b'"') => switch_to!(AfterDoctypeIdentifier(kind)),
                Char(b'>') => {
                    self.emit_error(Xml5Error::AbruptEndDoctypeIdentifier);
                    switch_to!(Data);
                    self.emit_doctype();
                }
                InterNeedle(start, len) => {
                    self.append_doctype_id((start, len));
                }
                _ => {
                    self.emit_error(Xml5Error::EofInDoctype);
                    self.emit_doctype();
                    self.emit_eof();
                }
            },
            DoctypeIdentifierSingleQuoted(kind) => match self.read_fast_until(&[b'\'', b'>']) {
                Char(b'\'') => switch_to!(AfterDoctypeIdentifier(kind)),
                Char(b'>') => {
                    self.emit_error(Xml5Error::AbruptEndDoctypeIdentifier);
                    switch_to!(Data);
                    self.emit_doctype();
                }
                InterNeedle(start, len) => {
                    self.append_doctype_id((start, len));
                }
                _ => {
                    self.emit_error(Xml5Error::EofInDoctype);
                    self.emit_doctype();
                    self.emit_eof();
                }
            },
            AfterDoctypeIdentifier(Public) => match next_char {
                Some(b'\t') | Some(b'\n') | Some(b' ') => {
                    switch_to!(BetweenDoctypePublicAndSystemIdentifiers)
                }
                Some(b'>') => {
                    switch_to!(Data);
                    self.emit_doctype();
                }
                Some(b'"') => {
                    self.emit_error(Xml5Error::MissingWhitespaceBetweenDoctypePublicAndSystem);
                    switch_to!(DoctypeIdentifierDoubleQuoted(System));
                }
                Some(b'\'') => {
                    self.emit_error(Xml5Error::MissingWhitespaceBetweenDoctypePublicAndSystem);
                    switch_to!(DoctypeIdentifierSingleQuoted(System));
                }
                None => {
                    self.emit_error(Xml5Error::EofInDoctype);
                    self.emit_doctype();
                    self.emit_eof();
                }
                _ => {
                    self.emit_error(Xml5Error::MissingQuoteBeforeIdentifier);
                    reconsume!(BogusDoctype);
                }
            },
            BetweenDoctypePublicAndSystemIdentifiers => match next_char {
                Some(b'\t') | Some(b'\n') | Some(b' ') => (),
                Some(b'"') => {
                    self.clear_doctype_id(System);
                    switch_to!(DoctypeIdentifierDoubleQuoted(System))
                }
                Some(b'\'') => {
                    self.clear_doctype_id(System);
                    switch_to!(DoctypeIdentifierSingleQuoted(System))
                }
                Some(b'>') => {
                    switch_to!(Data);
                    self.emit_doctype();
                }
                None => {
                    self.emit_error(Xml5Error::EofInDoctype);
                    self.emit_doctype();
                    self.emit_eof();
                }
                _ => {
                    self.emit_error(Xml5Error::MissingQuoteBeforeIdentifier);
                    reconsume!(BogusDoctype);
                }
            },
            AfterDoctypeIdentifier(System) => match next_char {
                Some(b'\t') | Some(b'\n') | Some(b' ') => (),
                Some(b'>') => {
                    self.emit_doctype();
                    switch_to!(Data);
                }
                _ => {
                    self.emit_error(Xml5Error::MissingQuoteBeforeIdentifier);
                    reconsume!(BogusDoctype);
                }
            },
            BogusDoctype => match next_char {
                Some(b'>') => {
                    switch_to!(Data);
                    self.emit_doctype();
                }
                None => {
                    self.emit_doctype();
                    self.emit_eof();
                }
                _ => (),
            },
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

    let find_r = find_str_in_xml(&mut xml, "r");
    assert_eq!((0, 6), find_r);
    assert_eq!(Char(b'r'), xml.read_fast_until(&[b'r']));
    let find_g = find_str_in_xml(&mut xml, "g");
    assert_eq!((6, 2), find_g);
    assert_eq!(Char(b'g'), xml.read_fast_until(&[b'g']));
    assert_eq!(EOF, xml.read_fast_until(&[b'r']));

    assert_eq!("TestSt".as_bytes(), &buf[find_r.0..find_r.0 + find_r.1]);
    assert_eq!("in".as_bytes(), &buf[find_g.0..find_g.0 + find_g.1]);
}

pub(crate) fn find_str_in_xml(xml: &mut Tokenizer<&[u8]>, needle: &str) -> (usize, usize) {
    match xml.read_fast_until(needle.as_bytes()) {
        InterNeedle(start, len) => (start, len),
        _ => panic!("Expected InterNeedle"),
    }
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
