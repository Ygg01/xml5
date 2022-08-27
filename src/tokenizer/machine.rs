use std::io::BufRead;

use FastRead::EOF;

use crate::errors::Xml5Error;
use crate::tokenizer::emitter::{DefaultEmitter, Emitter, XmlDeclAttr};
use crate::tokenizer::reader::FastRead::{Char, InterNeedle};
use crate::tokenizer::reader::{FastRead, Reader, SliceReader};
use crate::tokenizer::AttrValueKind::{DoubleQuoted, SingleQuoted, Unquoted};
use crate::tokenizer::Control::Eof;
use crate::tokenizer::DoctypeKind::{Public, System};
use crate::tokenizer::TokenState::*;
use crate::tokenizer::{Control, DeclQuote, SliceIterator};
use crate::Tokenizer;

impl Tokenizer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_str_reader(self, input: &str) -> SliceIterator<DefaultEmitter> {
        SliceIterator {
            state: self,
            reader: SliceReader::from_str(input),
            emitter: DefaultEmitter::default(),
        }
    }

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

        macro_rules! append_curr_char {
            ($meth:ident) => {{
                let tmp = reader.append_curr_char();
                emitter.$meth(tmp, tmp + 1);
            }};
        }

        match self.state {
            Data => {
                amt = 0;
                match reader.read_fast_until(&[b'<', b'&']) {
                    Char(b'&') => switch_to!(CharRefInData(Unquoted)),
                    Char(b'<') => switch_to!(TagOpen),
                    InterNeedle(start, end) => emitter.emit_chars(start, end),
                    _ => emitter.emit_eof(),
                }
            }
            TagOpen => match next_char {
                Some(b'/') => switch_to!(EndTagOpen),
                Some(b'?') => switch_to!(Pi),
                Some(b'!') => switch_to!(MarkupDecl),
                None | Some(b'\t') | Some(b'\n') | Some(b' ') | Some(b':') | Some(b'<')
                | Some(b'>') => {
                    emitter.emit_error(Xml5Error::UnexpectedSymbolOrEof(next_char));
                    emitter.emit_chars_now(b'<');
                    switch_to!(Data);
                }
                Some(_) => {
                    emitter.create_tag();
                    reconsume!(TagName);
                }
            },
            EndTagOpen => match next_char {
                Some(b'>') => {
                    emitter.emit_tag();
                    switch_to!(Data);
                }
                None | Some(b'\t') | Some(b'\n') | Some(b' ') | Some(b':') | Some(b'<') => {
                    emitter.emit_error(Xml5Error::UnexpectedSymbolOrEof(next_char));
                    emitter.emit_chars_now("</");
                    reconsume!(Data);
                }
                Some(_) => {
                    emitter.create_end_tag();
                    reconsume!(EndTagName);
                }
            },
            EndTagName => {
                amt = 0;
                match reader.read_fast_until(&[b'\t', b'\n', b' ', b'/', b'>']) {
                    Char(b'\t') | Char(b'\n') | Char(b' ') => {
                        switch_to!(EndTagNameAfter);
                    }
                    Char(b'/') => {
                        emitter.emit_error(Xml5Error::UnexpectedSymbol('/'));
                        switch_to!(EndTagNameAfter);
                    }
                    Char(b'>') => {
                        emitter.emit_end_tag();
                        switch_to!(Data);
                    }
                    InterNeedle(start, end) => emitter.append_tag(start, end),
                    _ => {
                        emitter.emit_error(Xml5Error::UnexpectedEof);
                    }
                }
            }
            EndTagNameAfter => match next_char {
                Some(b'>') => {
                    emitter.emit_end_tag();
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
            TagName => {
                amt = 0;
                match reader.read_fast_until(&[b'\t', b'\n', b' ', b'>', b'/']) {
                    Char(b'\t') | Char(b'\n') | Char(b' ') => switch_to!(TagAttrNameBefore),
                    Char(b'>') => {
                        emitter.emit_tag();
                        switch_to!(Data);
                    }
                    Char(b'/') => {
                        emitter.set_empty_tag();
                        switch_to!(EmptyTag);
                    }
                    InterNeedle(start, end) => {
                        emitter.append_tag(start, end);
                    }
                    _ => {
                        emitter.emit_error(Xml5Error::EofInTag);
                        emitter.emit_tag();
                        reconsume!(Data);
                    }
                }
            }
            EmptyTag => match next_char {
                Some(b'>') => {
                    emitter.emit_tag();
                    switch_to!(Data);
                }
                _ => reconsume!(TagAttrValueBefore),
            },
            TagAttrNameBefore => match next_char {
                Some(b'\t') | Some(b'\n') | Some(b' ') => (),
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
                Some(_) => {
                    emitter.create_attr();
                    append_curr_char!(attr_values);
                    switch_to!(TagAttrName);
                }
            },
            TagAttrName => {
                amt = 0;
                match reader.read_fast_until(&[b'\t', b'\n', b' ', b'=', b'>', b'/']) {
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
                }
            }
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
                Some(_) => {
                    emitter.create_attr();
                    append_curr_char!(attr_values);
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
                Some(_) => {
                    append_curr_char!(attr_values);
                    switch_to!(TagAttrValue(Unquoted));
                }
            },
            TagAttrValue(DoubleQuoted) => {
                amt = 0;
                match reader.read_fast_until(&[b'&', b'"']) {
                    Char(b'"') => switch_to!(TagAttrNameBefore),
                    Char(_) => switch_to!(CharRefInData(DoubleQuoted)),
                    InterNeedle(start, end) => emitter.attr_values(start, end),
                    EOF => {
                        emitter.emit_error(Xml5Error::EofInTag);
                        emitter.emit_tag();
                        reconsume!(Data);
                    }
                }
            }
            TagAttrValue(SingleQuoted) => {
                amt = 0;
                match reader.read_fast_until(&[b'&', b'\'']) {
                    Char(b'\'') => switch_to!(TagAttrNameBefore),
                    Char(_) => switch_to!(CharRefInData(DoubleQuoted)),
                    InterNeedle(start, end) => emitter.attr_values(start, end),
                    EOF => {
                        emitter.emit_error(Xml5Error::EofInTag);
                        emitter.emit_tag();
                        reconsume!(Data);
                    }
                }
            }
            TagAttrValue(Unquoted) => {
                amt = 0;
                match reader.read_fast_until(&[b'\t', b'\n', b' ', b'&', b'>']) {
                    Char(b'\t') | Char(b'\n') | Char(b' ') => switch_to!(TagAttrNameBefore),
                    Char(b'&') => switch_to!(CharRefInData(Unquoted)),
                    Char(_) => {
                        emitter.emit_tag();
                        switch_to!(Data);
                    }
                    InterNeedle(start, end) => emitter.attr_values(start, end),
                    EOF => {
                        emitter.emit_error(Xml5Error::EofInTag);
                        emitter.emit_tag();
                        reconsume!(Data);
                    }
                }
            }
            Pi => {
                amt = 0;
                if reader.try_read_slice_exact("xml") {
                    switch_to!(XmlDecl);
                } else {
                    match next_char {
                        None | Some(b' ') | Some(b'\n') | Some(b'\t') => {
                            emitter.emit_error(Xml5Error::UnexpectedSymbolOrEof(next_char));
                            reconsume!(BogusComment);
                        }
                        Some(_) => {
                            emitter.create_pi_tag();
                            reconsume!(PiTarget);
                        }
                    }
                }
            }
            XmlDecl => match next_char {
                Some(b'\t') | Some(b'\n') | Some(b' ') => {}
                Some(b'v') | Some(b'e') | Some(b's') => {
                    reconsume!(XmlDeclAttrName)
                }
                Some(_) => {
                    // TODO
                    // emitter.pi_target(xml)
                    emitter.emit_error(Xml5Error::InvalidXmlDeclaration);
                }
                None => {
                    // TODO
                    // emitter.pi_target()
                    emitter.emit_error(Xml5Error::EofInXmlDeclaration);
                }
            },
            XmlDeclAttrName => {
                amt = 0;
                if reader.try_read_slice_exact("version") {
                    emitter.set_xml_declaration(XmlDeclAttr::Version);
                    switch_to!(XmlDeclAttrNameAfter);
                } else if reader.try_read_slice_exact("encoding") {
                    emitter.set_xml_declaration(XmlDeclAttr::Encoding);
                    switch_to!(XmlDeclAttrNameAfter);
                } else if reader.try_read_slice_exact("standalone") {
                    emitter.set_xml_declaration(XmlDeclAttr::Standalone);
                    switch_to!(XmlDeclAttrNameAfter);
                } else {
                    emitter.emit_error(Xml5Error::InvalidXmlDeclaration);
                    // TODO
                    // emitter.pi_target()
                    switch_to!(PiTarget);
                }
            }
            XmlDeclAttrNameAfter => match next_char {
                Some(b'\t') | Some(b'\n') | Some(b' ') => {}
                Some(b'=') => switch_to!(XmlDeclAttrValueBefore),
                Some(_) => reconsume!(PiTarget),
                None => {
                    emitter.emit_error(Xml5Error::EofInXmlDeclaration);
                }
            },
            XmlDeclAttrValueBefore => match next_char {
                Some(b'\t') | Some(b'\n') | Some(b' ') => {}
                Some(b'\'') => switch_to!(XmlDeclAttrValue(DeclQuote::SingleQuoted)),
                Some(b'"') => switch_to!(XmlDeclAttrValue(DeclQuote::DoubleQuoted)),
                Some(_) => reconsume!(PiTarget),
                None => {
                    emitter.emit_error(Xml5Error::EofInXmlDeclaration);
                }
            },
            XmlDeclAttrValue(DeclQuote::DoubleQuoted) => {
                amt = 0;
                match reader.read_fast_until(&[b'"', b'?']) {
                    Char(b'"') => {
                        switch_to!(XmlDecl)
                    }
                    Char(b'?') => {
                        // emitter.emit_xml_decl();
                        emitter.emit_error(Xml5Error::AbruptClosingXmlDeclaration);
                        switch_to!(PiAfter);
                    }
                    InterNeedle(start, end) => emitter.emit_decl_value(start, end),
                    EOF | _ => {
                        emitter.emit_error(Xml5Error::EofInXmlDeclaration);
                    }
                }
            }
            XmlDeclAttrValue(DeclQuote::SingleQuoted) => {
                amt = 0;
                match reader.read_fast_until(&[b'\'', b'?']) {
                    Char(b'\'') => switch_to!(XmlDecl),
                    Char(b'?') => {
                        emitter.emit_error(Xml5Error::AbruptClosingXmlDeclaration);
                        switch_to!(PiAfter);
                    }
                    InterNeedle(start, end) => emitter.emit_decl_value(start, end),
                    EOF | _ => {
                        emitter.emit_error(Xml5Error::EofInXmlDeclaration);
                    }
                }
            }
            XmlDeclAfter => {}
            PiTarget => {
                amt = 0;
                match reader.read_fast_until(&[b'\t', b'\n', b' ']) {
                    Char(b'\t') | Char(b'\n') | Char(b' ') => switch_to!(PiTargetAfter),
                    Char(b'?') => switch_to!(PiAfter),
                    InterNeedle(start, end) => emitter.pi_target(start, end),
                    _ => {
                        emitter.emit_pi();
                        emitter.emit_error(Xml5Error::UnexpectedEof);
                        reconsume!(Data);
                    }
                }
            }
            PiTargetAfter => match next_char {
                Some(b'\t') | Some(b'\n') | Some(b' ') => {}
                _ => reconsume!(PiData),
            },
            PiData => {
                amt = 0;
                match reader.read_fast_until(&[b'?']) {
                    Char(b'?') => switch_to!(PiAfter),
                    InterNeedle(start, end) => emitter.pi_data(start, end),
                    _ => {
                        emitter.emit_error(Xml5Error::UnexpectedEof);
                        emitter.emit_pi();
                        reconsume!(Data);
                    }
                }
            }
            PiAfter => match next_char {
                Some(b'>') => {
                    emitter.emit_pi();
                    switch_to!(Data);
                }
                Some(b'?') => append_curr_char!(pi_target),
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
                    emitter.to_comment_now(b'-');
                    reconsume!(Comment);
                }
            },
            Comment => {
                amt = 0;
                match reader.read_fast_until(&[b'<', b'-']) {
                    InterNeedle(start, end) => {
                        emitter.to_comment(start, end);
                    }
                    Char(b'<') => {
                        append_curr_char!(to_comment);
                        switch_to!(CommentLessThan)
                    }
                    Char(b'-') => switch_to!(CommentEndDash),
                    _ => {
                        emitter.emit_error(Xml5Error::EofInComment);
                        emitter.emit_comment();
                        emitter.emit_eof();
                    }
                }
            }
            CommentLessThan => match next_char {
                Some(b'!') => {
                    append_curr_char!(to_comment);
                    switch_to!(CommentLessThanBang);
                }
                Some(b'<') => {
                    append_curr_char!(to_comment);
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
                    emitter.to_comment_now(b'-');
                    reconsume!(Comment);
                }
            },
            CommentEnd => match next_char {
                Some(b'>') => {
                    switch_to!(Data);
                    emitter.emit_comment();
                }
                Some(b'!') => switch_to!(CommentEndBang),
                Some(b'-') => append_curr_char!(to_comment),
                None => {
                    emitter.emit_error(Xml5Error::EofInComment);
                    emitter.emit_comment();
                    emitter.emit_eof();
                }
                _ => {
                    emitter.to_comment_now("--");
                    reconsume!(Comment);
                }
            },
            CommentEndBang => match next_char {
                Some(b'-') => {
                    emitter.to_comment_now("-!");
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
                    emitter.to_comment_now("--");
                    reconsume!(Comment)
                }
            },
            Cdata => {
                amt = 0;
                match reader.read_fast_until(&[b']']) {
                    Char(b']') => switch_to!(CdataBracket),
                    InterNeedle(start, end) => emitter.emit_chars(start, end),
                    EOF | _ => {
                        emitter.emit_error(Xml5Error::EofInCdata);
                        reconsume!(Data);
                    }
                }
            }
            CdataBracket => match next_char {
                Some(b']') => switch_to!(CdataEnd),
                None => {
                    emitter.emit_error(Xml5Error::EofInCdata);
                    reconsume!(Data);
                }
                Some(_) => {
                    emitter.emit_chars_now(b']');
                    append_curr_char!(emit_chars);
                    switch_to!(CdataBracket);
                }
            },
            CdataEnd => match next_char {
                Some(b'>') => switch_to!(Data),
                Some(b']') => emitter.emit_chars_now(b']'),
                None => {
                    emitter.emit_error(Xml5Error::EofInCdata);
                    reconsume!(Data);
                }
                Some(c) => {
                    emitter.emit_chars_now("]]");
                    emitter.emit_chars_now(c);
                    switch_to!(Cdata);
                }
            },
            BogusComment => {
                amt = 0;
                match reader.read_fast_until(&[b'>']) {
                    Char(_) => {
                        switch_to!(Data);
                    }
                    InterNeedle(start, end) => emitter.to_comment(start, end),
                    _ => {
                        emitter.emit_comment();
                        emitter.emit_eof();
                    }
                }
            }
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
                    emitter.doctype_name_now(chr);
                    switch_to!(DoctypeName);
                }
            },
            DoctypeName => match next_char {
                Some(b'\t') | Some(b'\n') | Some(b' ') => switch_to!(AfterDoctypeName(0)),
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
                    emitter.doctype_name_now(chr);
                    switch_to!(DoctypeName);
                }
                _ => {}
            },
            AfterDoctypeName(depth) => match reader.read_fast_until(&[b'[', b']', b'>']) {
                Char(b'[') => switch_to!(AfterDoctypeName(depth + 1)),
                Char(b']') => {
                    if depth == 0 {
                        switch_to!(BogusDoctype);
                    } else {
                        switch_to!(AfterDoctypeName(depth - 1));
                    }
                }
                Char(b'>') if depth == 0 => {
                    emitter.emit_doctype();
                    switch_to!(Data);
                }
                EOF => {
                    emitter.emit_doctype();
                    emitter.emit_error(Xml5Error::EofInDoctype);
                    emitter.emit_eof();
                }
                _ => {}
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
