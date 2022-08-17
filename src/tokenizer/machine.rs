use std::io::BufRead;

use FastRead::EOF;

use crate::errors::Xml5Error;
use crate::tokenizer::emitter::Emitter;
use crate::tokenizer::reader::FastRead::{Char, InterNeedle};
use crate::tokenizer::reader::{FastRead, Reader};
use crate::tokenizer::AttrValueKind::{DoubleQuoted, SingleQuoted, Unquoted};
use crate::tokenizer::Control;
use crate::tokenizer::Control::Eof;
use crate::tokenizer::DoctypeKind::{Public, System};
use crate::tokenizer::TokenState::*;
use crate::Tokenizer;

impl Tokenizer {
    #[inline]
    pub(crate) fn next_state<'r, E, R>(&mut self, reader: &mut R, emitter: &mut E) -> Control
    where
        R: Reader<'r>,
        E: Emitter,
    {
        let mut amt = 1;
        let next_char = match reader.peek_byte() {
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
            Data => match reader.read_fast_until(&[b'<', b'&']) {
                Char(b'&') => switch_to!(CharRefInData(Unquoted)),
                Char(b'<') => switch_to!(TagOpen),
                InterNeedle(start, len) => emitter.emit_chars((start, len)),
                _ => emitter.emit_eof(),
            },
            TagOpen => match next_char {
                Some(b'/') => switch_to!(EndTagOpen),
                Some(b'?') => switch_to!(Pi),
                Some(b'!') => switch_to!(MarkupDecl),
                None | Some(b'\t') | Some(b'\n') | Some(b' ') | Some(b':') | Some(b'<')
                | Some(b'>') => {
                    emitter.emit_error(Xml5Error::UnexpectedSymbolOrEof(next_char));
                    emitter.emit_chars(b'<');
                    switch_to!(Data);
                }
                Some(c) => {
                    emitter.create_tag();
                    emitter.append_tag(c);
                    switch_to!(TagName);
                }
            },
            EndTagOpen => match next_char {
                Some(b'>') => {
                    emitter.emit_tag();
                    switch_to!(Data);
                }
                None | Some(b'\t') | Some(b'\n') | Some(b' ') | Some(b':') | Some(b'<') => {
                    emitter.emit_error(Xml5Error::UnexpectedSymbolOrEof(next_char));
                    emitter.emit_chars("</");
                    reconsume!(Data);
                }
                Some(c) => {
                    emitter.create_end_tag();
                    emitter.append_tag(c);
                    switch_to!(EndTagName);
                }
            },
            EndTagName => match reader.read_fast_until(&[b'\t', b'\n', b' ', b'/', b'>']) {
                Char(b'\t') | Char(b'\n') | Char(b' ') => {
                    switch_to!(EndTagNameAfter);
                }
                Char(b'/') => {
                    emitter.emit_error(Xml5Error::UnexpectedSymbol('/'));
                    switch_to!(EndTagNameAfter);
                }
                Char(b'>') => {
                    emitter.append_tag(b'>');
                    switch_to!(Data);
                }
                InterNeedle(start, len) => emitter.append_tag((start, len)),
                _ => {
                    emitter.emit_error(Xml5Error::UnexpectedEof);
                }
            },
            EndTagNameAfter => match next_char {
                Some(b'>') => {
                    emitter.emit_tag();
                    switch_to!(Data);
                }
                Some(b' ') | Some(b'\n') | Some(b'\t') => {}
                None => {
                    emitter.emit_error(Xml5Error::UnexpectedSymbolOrEof(None));
                    reconsume!(Data);
                }
                Some(x) => {
                    emitter.emit_error(Xml5Error::UnexpectedSymbol(x as char));
                }
            },
            TagName => match reader.read_fast_until(&[b'\t', b'\n', b' ', b'>', b'/']) {
                Char(b'\t') | Char(b'\n') | Char(b' ') => switch_to!(TagAttrNameBefore),
                Char(b'>') => {
                    emitter.emit_tag();
                    switch_to!(Data);
                }
                Char(b'/') => {
                    emitter.set_empty_tag();
                    switch_to!(EmptyTag);
                }
                InterNeedle(start, len) => {
                    emitter.append_tag((start, len));
                }
                _ => {
                    emitter.emit_error(Xml5Error::EofInTag);
                    emitter.emit_tag();
                    reconsume!(Data);
                }
            },
            EmptyTag => match next_char {
                Some(b'>') => {
                    emitter.emit_tag();
                    switch_to!(Data);
                }
                _ => reconsume!(TagAttrValueBefore),
            },
            TagAttrNameBefore => match next_char {
                Some(b't') | Some(b't') | Some(b't') => (),
                Some(b'>') => {
                    emitter.emit_tag();
                    switch_to!(Data);
                }
                Some(b'/') => {
                    emitter.set_empty_tag();
                    switch_to!(EmptyTag);
                }
                Some(b':') => emitter.emit_error(Xml5Error::ColonBeforeAttrName),
                None => {
                    emitter.emit_error(Xml5Error::EofInTag);
                    emitter.emit_tag();
                    reconsume!(Data);
                }
                Some(c) => {
                    emitter.create_attr();
                    emitter.push_attr_values(c);
                    switch_to!(TagAttrName);
                }
            },
            TagAttrName => match reader.read_fast_until(&[b'\t', b'\n', b' ', b'=', b'>', b'/']) {
                Char(b'\t') | Char(b'\n') | Char(b' ') => switch_to!(TagAttrValueBefore),
                Char(b'=') => switch_to!(TagAttrValueBefore),
                Char(b'>') => {
                    emitter.emit_tag();
                    switch_to!(Data);
                }
                Char(b'/') => {
                    emitter.set_empty_tag();
                    switch_to!(EmptyTag);
                }
                EOF | _ => {
                    emitter.emit_error(Xml5Error::EofInTag);
                    emitter.emit_tag();
                    reconsume!(Data);
                }
            },
            TagAttrNameAfter => match next_char {
                Some(b'\t') | Some(b'\n') | Some(b' ') => (),
                Some(b'=') => switch_to!(TagAttrValueBefore),
                Some(b'>') => {
                    emitter.emit_tag();
                    switch_to!(EmptyTag);
                }
                Some(b'/') => {
                    emitter.set_empty_tag();
                    switch_to!(EmptyTag);
                }
                None => {
                    emitter.emit_error(Xml5Error::EofInTag);
                    emitter.emit_tag();
                    reconsume!(Data);
                }
                Some(c) => {
                    emitter.create_attr();
                    emitter.push_attr_name(c);
                    switch_to!(TagAttrName)
                }
            },
            TagAttrValueBefore => match next_char {
                Some(b'\t') | Some(b'\n') | Some(b' ') => (),
                Some(b'"') => switch_to!(TagAttrValue(DoubleQuoted)),
                Some(b'\'') => switch_to!(TagAttrValue(SingleQuoted)),
                Some(b'&') => reconsume!(TagAttrValue(Unquoted)),
                Some(b'>') => {
                    emitter.emit_tag();
                    switch_to!(Data);
                }
                None => {
                    emitter.emit_error(Xml5Error::EofInTag);
                    emitter.emit_tag();
                    reconsume!(Data);
                }
                Some(c) => {
                    emitter.push_attr_values(c);
                    switch_to!(TagAttrValue(Unquoted));
                }
            },
            TagAttrValue(DoubleQuoted) => match reader.read_fast_until(&[b'&', b'"']) {
                Char(b'"') => switch_to!(TagAttrNameBefore),
                Char(_) => switch_to!(CharRefInData(DoubleQuoted)),
                InterNeedle(start, len) => emitter.push_attr_values((start, len)),
                EOF => {
                    emitter.emit_error(Xml5Error::EofInTag);
                    emitter.emit_tag();
                    reconsume!(Data);
                }
            },
            TagAttrValue(SingleQuoted) => match reader.read_fast_until(&[b'&', b'\'']) {
                Char(b'\'') => switch_to!(TagAttrNameBefore),
                Char(_) => switch_to!(CharRefInData(DoubleQuoted)),
                InterNeedle(start, len) => emitter.push_attr_values((start, len)),
                EOF => {
                    emitter.emit_error(Xml5Error::EofInTag);
                    emitter.emit_tag();
                    reconsume!(Data);
                }
            },
            TagAttrValue(Unquoted) => {
                match reader.read_fast_until(&[b'\t', b'\n', b' ', b'&', b'>']) {
                    Char(b'\t') | Char(b'\n') | Char(b' ') => switch_to!(TagAttrNameBefore),
                    Char(b'&') => switch_to!(CharRefInData(Unquoted)),
                    Char(_) => {
                        emitter.emit_tag();
                        switch_to!(Data);
                    }
                    InterNeedle(start, len) => emitter.push_attr_values((start, len)),
                    EOF => {
                        emitter.emit_error(Xml5Error::EofInTag);
                        emitter.emit_tag();
                        reconsume!(Data);
                    }
                }
            }
            Pi => match next_char {
                None | Some(b' ') | Some(b'\n') | Some(b'\t') => {
                    emitter.emit_error(Xml5Error::UnexpectedSymbolOrEof(next_char));
                    reconsume!(BogusComment);
                }
                Some(c) => {
                    emitter.create_pi_tag();
                    emitter.append_pi_data(c);
                    switch_to!(PiTarget);
                }
            },
            PiTarget => match reader.read_fast_until(&[b'\t', b'\n', b' ']) {
                Char(b'\t') | Char(b'\n') | Char(b' ') => switch_to!(PiTargetAfter),
                Char(b'?') => switch_to!(PiAfter),
                InterNeedle(start, len) => emitter.append_pi_target((start, len)),
                _ => {
                    emitter.emit_pi();
                    emitter.emit_error(Xml5Error::UnexpectedEof);
                    reconsume!(Data);
                }
            },
            PiTargetAfter => match next_char {
                Some(b'\t') | Some(b'\n') | Some(b' ') => {}
                _ => reconsume!(PiData),
            },
            PiData => match reader.read_fast_until(&[b'?']) {
                Char(b'?') => switch_to!(PiAfter),
                InterNeedle(start, end) => emitter.append_pi_data((start, end)),
                _ => {
                    emitter.emit_error(Xml5Error::UnexpectedEof);
                    emitter.emit_pi();
                    reconsume!(Data);
                }
            },
            PiAfter => match next_char {
                Some(b'>') => {
                    emitter.emit_pi();
                    switch_to!(Data);
                }
                Some(b'?') => emitter.append_pi_data(b'?'),
                _ => reconsume!(PiData),
            },
            MarkupDecl => {
                if reader.try_read_slice_exact("--") {
                    emitter.create_comment_token();
                    switch_to!(CommentStart)
                } else if reader.try_read_slice("DOCTYPE", false) {
                    switch_to!(Doctype)
                } else if reader.try_read_slice_exact("[CDATA[") {
                    switch_to!(Cdata)
                } else {
                    emitter.emit_error(Xml5Error::IncorrectlyOpenedComment);
                    switch_to!(BogusComment)
                }
            }
            CommentStart => match next_char {
                Some(b'-') => switch_to!(CommentStartDash),
                Some(b'>') => {
                    emitter.emit_error(Xml5Error::AbruptClosingEmptyComment);
                    switch_to!(Data);
                    emitter.emit_comment();
                }
                _ => reconsume!(Comment),
            },
            CommentStartDash => match next_char {
                Some(b'-') => switch_to!(CommentEnd),
                Some(b'>') => {
                    emitter.emit_error(Xml5Error::AbruptClosingEmptyComment);
                    switch_to!(Data);
                    emitter.emit_comment();
                }
                None => {
                    emitter.emit_error(Xml5Error::EofInComment);
                    emitter.emit_comment();
                    emitter.emit_eof();
                }
                _ => {
                    emitter.append_to_comment(b'-');
                    reconsume!(Comment);
                }
            },
            Comment => match reader.read_fast_until(&[b'<', b'-']) {
                InterNeedle(start, end) => {
                    emitter.append_to_comment((start, end));
                }
                Char(b'<') => {
                    emitter.append_to_comment(b'<');
                    switch_to!(CommentLessThan)
                }
                Char(b'-') => switch_to!(CommentEndDash),
                _ => {
                    emitter.emit_error(Xml5Error::EofInComment);
                    emitter.emit_comment();
                    emitter.emit_eof();
                }
            },
            CommentLessThan => match next_char {
                Some(b'!') => {
                    emitter.append_to_comment(b'!');
                    switch_to!(CommentLessThanBang);
                }
                Some(b'<') => {
                    emitter.append_to_comment(b'<');
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
                    emitter.emit_error(Xml5Error::EofInComment);
                    emitter.emit_comment();
                    emitter.emit_eof();
                }
                _ => {
                    emitter.append_to_comment(b'-');
                    reconsume!(Comment);
                }
            },
            CommentEnd => match next_char {
                Some(b'>') => {
                    switch_to!(Data);
                    emitter.emit_comment();
                }
                Some(b'!') => switch_to!(CommentEndBang),
                Some(b'-') => emitter.append_to_comment(b'-'),
                None => {
                    emitter.emit_error(Xml5Error::EofInComment);
                    emitter.emit_comment();
                    emitter.emit_eof();
                }
                _ => {
                    emitter.append_to_comment("--");
                    reconsume!(Comment);
                }
            },
            CommentEndBang => match next_char {
                Some(b'-') => {
                    emitter.append_to_comment("-!");
                    switch_to!(CommentEndDash);
                }
                Some(b'>') => {
                    emitter.emit_error(Xml5Error::GreaterThanInComment);
                    switch_to!(Data);
                    emitter.emit_comment();
                }
                None => {
                    emitter.emit_error(Xml5Error::EofInComment);
                    emitter.emit_comment();
                    emitter.emit_eof();
                }
                _ => {
                    emitter.append_to_comment("--");
                    reconsume!(Comment)
                }
            },
            Cdata => match reader.read_fast_until(&[b']']) {
                Char(b']') => switch_to!(CdataBracket),
                InterNeedle(start, end) => emitter.emit_chars((start, end)),
                EOF | _ => {
                    emitter.emit_error(Xml5Error::EofInCdata);
                    reconsume!(Data);
                }
            },
            CdataBracket => match next_char {
                Some(b']') => switch_to!(CdataEnd),
                None => {
                    emitter.emit_error(Xml5Error::EofInCdata);
                    reconsume!(Data);
                }
                Some(c) => {
                    emitter.emit_chars(b']');
                    emitter.emit_chars(c);
                    switch_to!(CdataBracket);
                }
            },
            CdataEnd => match next_char {
                Some(b'>') => switch_to!(Data),
                Some(b']') => emitter.emit_chars(b']'),
                None => {
                    emitter.emit_error(Xml5Error::EofInCdata);
                    reconsume!(Data);
                }
                Some(c) => {
                    emitter.emit_chars("]]");
                    emitter.emit_chars(c);
                    switch_to!(Cdata);
                }
            },
            BogusComment => match reader.read_fast_until(&[b'>']) {
                Char(_) => {
                    switch_to!(Data);
                }
                InterNeedle(start, len) => emitter.append_to_comment((start, len)),
                _ => {
                    emitter.emit_comment();
                    emitter.emit_eof();
                }
            },
            Doctype => match next_char {
                Some(b'\t') | Some(b'\n') | Some(b' ') => switch_to!(BeforeDoctypeName),
                None => {
                    emitter.emit_error(Xml5Error::EofInDoctype);
                    emitter.emit_doctype();
                    emitter.emit_eof();
                }
                _ => {
                    emitter.emit_error(Xml5Error::MissingWhitespaceDoctype);
                    reconsume!(BeforeDoctypeName);
                }
            },
            BeforeDoctypeName => match next_char {
                Some(b'\t') | Some(b'\n') | Some(b' ') => (),
                Some(b'>') => {
                    emitter.emit_error(Xml5Error::MissingDoctypeName);
                    emitter.emit_doctype();
                    switch_to!(Data);
                }
                None => {
                    emitter.emit_error(Xml5Error::EofInDoctype);
                    emitter.emit_doctype();
                    emitter.emit_eof();
                }
                Some(x) => {
                    emitter.create_doctype();
                    let chr = if x.is_ascii_uppercase() {
                        x.to_ascii_lowercase()
                    } else {
                        x
                    };
                    emitter.append_doctype_name(chr);
                    switch_to!(DoctypeName);
                }
            },
            DoctypeName => match next_char {
                Some(b'\t') | Some(b'\n') | Some(b' ') => switch_to!(AfterDoctypeName),
                Some(b'>') => {
                    emitter.emit_doctype();
                    switch_to!(Data);
                }
                None => {
                    emitter.emit_error(Xml5Error::EofInDoctype);
                    emitter.emit_doctype();
                    emitter.emit_eof();
                }
                Some(x) => {
                    let chr = if x.is_ascii_uppercase() {
                        x.to_ascii_lowercase()
                    } else {
                        x
                    };
                    emitter.append_doctype_name(chr);
                    switch_to!(DoctypeName);
                }
            },
            AfterDoctypeName => match next_char {
                Some(b'\t') | Some(b'\n') | Some(b' ') => (),
                Some(b'>') => {
                    switch_to!(Data);
                    emitter.emit_doctype();
                }
                None => {
                    emitter.emit_error(Xml5Error::EofInDoctype);
                    emitter.emit_doctype();
                    emitter.emit_eof();
                }
                _ => {
                    if reader.try_read_slice("PUBLIC", false) {
                        switch_to!(AfterDoctypeKeyword(Public))
                    } else if reader.try_read_slice("SYSTEM", false) {
                        switch_to!(AfterDoctypeKeyword(System))
                    } else {
                        emitter.emit_error(Xml5Error::InvalidCharactersInAfterDoctypeName);
                        switch_to!(BogusComment);
                    }
                }
            },
            AfterDoctypeKeyword(Public) => match next_char {
                Some(b'\t') | Some(b'\n') | Some(b' ') => {
                    switch_to!(BeforeDoctypeIdentifier(Public))
                }
                Some(b'"') => {
                    emitter.emit_error(Xml5Error::MissingWhitespaceAfterDoctypeKeyword);
                    emitter.clear_doctype_id(Public);
                    switch_to!(DoctypeIdentifierDoubleQuoted(Public));
                }
                Some(b'\'') => {
                    emitter.emit_error(Xml5Error::MissingWhitespaceAfterDoctypeKeyword);
                    emitter.clear_doctype_id(Public);
                    switch_to!(DoctypeIdentifierSingleQuoted(Public));
                }
                Some(b'>') => {
                    emitter.emit_error(Xml5Error::MissingWhitespaceAfterDoctypeKeyword);
                    emitter.clear_doctype_id(Public);
                    switch_to!(Data);
                }
                None => {
                    emitter.emit_error(Xml5Error::EofInDoctype);
                    emitter.emit_doctype();
                    emitter.emit_eof();
                }
                _ => {
                    emitter.emit_error(Xml5Error::MissingQuoteBeforeIdentifier);
                    reconsume!(BogusDoctype);
                }
            },
            AfterDoctypeKeyword(System) => match next_char {
                Some(b'\t') | Some(b'\n') | Some(b' ') => {
                    switch_to!(BeforeDoctypeIdentifier(System))
                }
                Some(b'"') => {
                    emitter.emit_error(Xml5Error::MissingWhitespaceAfterDoctypeKeyword);
                    emitter.clear_doctype_id(System);
                    switch_to!(DoctypeIdentifierDoubleQuoted(System));
                }
                Some(b'\'') => {
                    emitter.emit_error(Xml5Error::MissingWhitespaceAfterDoctypeKeyword);
                    emitter.clear_doctype_id(System);
                    switch_to!(DoctypeIdentifierSingleQuoted(System));
                }
                Some(b'>') => {
                    emitter.emit_error(Xml5Error::MissingWhitespaceAfterDoctypeKeyword);
                    emitter.clear_doctype_id(System);
                    switch_to!(Data);
                }
                None => {
                    emitter.emit_error(Xml5Error::EofInDoctype);
                    emitter.emit_doctype();
                    emitter.emit_eof();
                }
                _ => {
                    emitter.emit_error(Xml5Error::MissingQuoteBeforeIdentifier);
                    reconsume!(BogusDoctype);
                }
            },
            BeforeDoctypeIdentifier(kind) => match next_char {
                Some(b'\t') | Some(b'\n') | Some(b' ') => (),
                Some(b'"') => {
                    emitter.clear_doctype_id(kind);
                    switch_to!(DoctypeIdentifierDoubleQuoted(kind));
                }
                Some(b'\'') => {
                    emitter.clear_doctype_id(kind);
                    switch_to!(DoctypeIdentifierSingleQuoted(kind));
                }
                Some(b'>') => {
                    emitter.emit_error(Xml5Error::MissingDoctypeIdentifier);
                    switch_to!(Data);
                    emitter.emit_doctype();
                }
                None => {
                    emitter.emit_error(Xml5Error::EofInDoctype);
                    emitter.emit_doctype();
                    emitter.emit_eof();
                }
                _ => {
                    emitter.emit_error(Xml5Error::MissingQuoteBeforeIdentifier);
                    reconsume!(BogusDoctype);
                }
            },
            DoctypeIdentifierDoubleQuoted(kind) => match reader.read_fast_until(&[b'"', b'>']) {
                Char(b'"') => switch_to!(AfterDoctypeIdentifier(kind)),
                Char(b'>') => {
                    emitter.emit_error(Xml5Error::AbruptEndDoctypeIdentifier);
                    switch_to!(Data);
                    emitter.emit_doctype();
                }
                InterNeedle(start, len) => {
                    emitter.append_doctype_id((start, len));
                }
                _ => {
                    emitter.emit_error(Xml5Error::EofInDoctype);
                    emitter.emit_doctype();
                    emitter.emit_eof();
                }
            },
            DoctypeIdentifierSingleQuoted(kind) => match reader.read_fast_until(&[b'\'', b'>']) {
                Char(b'\'') => switch_to!(AfterDoctypeIdentifier(kind)),
                Char(b'>') => {
                    emitter.emit_error(Xml5Error::AbruptEndDoctypeIdentifier);
                    switch_to!(Data);
                    emitter.emit_doctype();
                }
                InterNeedle(start, len) => {
                    emitter.append_doctype_id((start, len));
                }
                _ => {
                    emitter.emit_error(Xml5Error::EofInDoctype);
                    emitter.emit_doctype();
                    emitter.emit_eof();
                }
            },
            AfterDoctypeIdentifier(Public) => match next_char {
                Some(b'\t') | Some(b'\n') | Some(b' ') => {
                    switch_to!(BetweenDoctypePublicAndSystemIdentifiers)
                }
                Some(b'>') => {
                    switch_to!(Data);
                    emitter.emit_doctype();
                }
                Some(b'"') => {
                    emitter.emit_error(Xml5Error::MissingWhitespaceBetweenDoctypePublicAndSystem);
                    switch_to!(DoctypeIdentifierDoubleQuoted(System));
                }
                Some(b'\'') => {
                    emitter.emit_error(Xml5Error::MissingWhitespaceBetweenDoctypePublicAndSystem);
                    switch_to!(DoctypeIdentifierSingleQuoted(System));
                }
                None => {
                    emitter.emit_error(Xml5Error::EofInDoctype);
                    emitter.emit_doctype();
                    emitter.emit_eof();
                }
                _ => {
                    emitter.emit_error(Xml5Error::MissingQuoteBeforeIdentifier);
                    reconsume!(BogusDoctype);
                }
            },
            BetweenDoctypePublicAndSystemIdentifiers => match next_char {
                Some(b'\t') | Some(b'\n') | Some(b' ') => (),
                Some(b'"') => {
                    emitter.clear_doctype_id(System);
                    switch_to!(DoctypeIdentifierDoubleQuoted(System))
                }
                Some(b'\'') => {
                    emitter.clear_doctype_id(System);
                    switch_to!(DoctypeIdentifierSingleQuoted(System))
                }
                Some(b'>') => {
                    switch_to!(Data);
                    emitter.emit_doctype();
                }
                None => {
                    emitter.emit_error(Xml5Error::EofInDoctype);
                    emitter.emit_doctype();
                    emitter.emit_eof();
                }
                _ => {
                    emitter.emit_error(Xml5Error::MissingQuoteBeforeIdentifier);
                    reconsume!(BogusDoctype);
                }
            },
            AfterDoctypeIdentifier(System) => match next_char {
                Some(b'\t') | Some(b'\n') | Some(b' ') => (),
                Some(b'>') => {
                    emitter.emit_doctype();
                    switch_to!(Data);
                }
                _ => {
                    emitter.emit_error(Xml5Error::MissingQuoteBeforeIdentifier);
                    reconsume!(BogusDoctype);
                }
            },
            BogusDoctype => match next_char {
                Some(b'>') => {
                    switch_to!(Data);
                    emitter.emit_doctype();
                }
                None => {
                    emitter.emit_doctype();
                    emitter.emit_eof();
                }
                _ => (),
            },
            CharRefInData(_) => {
                // TODO char ref
            }
        };
        reader.consume_bytes(amt);
        Control::Continue
    }
}
