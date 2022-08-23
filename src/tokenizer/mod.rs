use std::borrow::Cow;

use crate::errors::Xml5Error;
use crate::tokenizer::emitter::{Emitter, Mix, SpanTokens, Spans};
#[cfg(feature = "encoding")]
use crate::tokenizer::encoding::EncodingRef;
use crate::tokenizer::reader::{BuffReader, Reader, SliceReader};
use crate::tokenizer::TokenState::Cdata;
use crate::Token;
use crate::Token::Text;

mod decoding;
mod emitter;
#[cfg(feature = "encoding")]
mod encoding;
mod machine;
mod reader;

#[derive(Default)]
pub struct Tokenizer {
    /// which state is the tokenizer in
    state: TokenState,
    /// End of file reached - parsing stops
    eof: bool,
    /// encoding specified in the xml, or utf8 if none found
    #[cfg(feature = "encoding")]
    encoder_ref: EncodingRef,
    /// checks if xml5 could identify encoding
    #[cfg(feature = "encoding")]
    is_encoding_set: bool,
}

pub struct SliceIterator<'a, E> {
    state: Tokenizer,
    reader: SliceReader<'a>,
    emitter: E,
}

impl<'a, E> Iterator for SliceIterator<'a, E>
where
    E: Emitter<Output = SpanTokens>,
{
    type Item = Token<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let span = loop {
            if let Some(token) = self.emitter.pop_token() {
                break token;
            } else if !self.state.eof {
                match self.state.next_state(&mut self.reader, &mut self.emitter) {
                    Control::Continue => (),
                    Control::Eof => {
                        self.state.eof = true;
                        self.emitter.emit_eof();
                    }
                    _ => return None,
                }
            } else {
                return None;
            }
        };
        Some(match span {
            SpanTokens::EndTag(Some(sp)) => Token::end_tag(self.to_cow(sp)),
            SpanTokens::PiData { data, target } => {
                Token::pi_tag(self.to_cow(data), self.to_cow(target))
            }

            SpanTokens::Comment(text) => Token::comment(self.to_cow(text)),
            SpanTokens::Decl(text) => Token::declaration(self.to_cow(text)),
            SpanTokens::CData(text) => Token::cdata(self.to_cow(text)),

            SpanTokens::Text(text) => Token::text(self.to_cow(text)),
            SpanTokens::StartTag {
                name,
                attrs,
                self_close: true,
                ..
            } => Token::start_tag(self.to_cow(name), self.to_attrs(attrs)),
            SpanTokens::StartTag {
                name,
                attrs,
                self_close: false,
                ..
            } => Token::empty_tag(self.to_cow(name), self.to_attrs(attrs)),
            SpanTokens::Error(err) => Token::Error(err),
            SpanTokens::Eof => Token::Eof,
            SpanTokens::EndTag(None) => Token::auto_close_tag(),
        })
    }
}

impl<'a, E> SliceIterator<'a, E> {
    fn to_cow(&self, span: Spans) -> Cow<'a, [u8]> {
        if let Some((start, end)) = span.to_range() {
            Cow::Borrowed(&self.reader.slice[start..end])
        } else {
            let mut vec = vec![];
            for datum in span.data {
                match datum {
                    Mix::Owned(bytes) => vec.extend_from_slice(&bytes),
                    Mix::Range(s, e) => vec.extend_from_slice(&self.reader.slice[s..e]),
                }
            }
            Cow::Owned(vec)
        }
    }

    fn to_attrs(&self, attrs: Vec<(Spans, Spans)>) -> Vec<(Cow<'a, [u8]>, Cow<'a, [u8]>)> {
        let mut vec = vec![];
        for (name, val) in attrs {
            vec.push((self.to_cow(name), self.to_cow(val)));
        }
        vec
    }
}

pub struct BufIterator<'a, R, E> {
    state: Tokenizer,
    reader: BuffReader<'a, R>,
    emitter: E,
}

pub(crate) enum Control {
    Continue,
    Eof,
    Err(Xml5Error),
}

#[derive(Debug, Clone, Copy)]
#[doc(hidden)]
enum TokenState {
    Data,
    CharRefInData(AttrValueKind),
    TagOpen,
    EndTagOpen,
    EndTagName,
    EndTagNameAfter,
    Pi,
    PiTarget,
    PiTargetAfter,
    PiData,
    PiAfter,
    XmlDecl,
    XmlDeclAttrName,
    XmlDeclAttrNameAfter,
    XmlDeclAttrValueBefore,
    XmlDeclAttrValue(DeclQuote),
    XmlDeclAfter,
    MarkupDecl,
    CommentStart,
    CommentStartDash,
    Comment,
    CommentLessThan,
    CommentLessThanBang,
    CommentLessThanBangDash,
    CommentLessThanBangDashDash,
    CommentEnd,
    CommentEndDash,
    CommentEndBang,
    Cdata,
    CdataBracket,
    CdataEnd,
    BogusComment,
    TagName,
    EmptyTag,
    TagAttrNameBefore,
    TagAttrName,
    TagAttrNameAfter,
    TagAttrValueBefore,
    TagAttrValue(AttrValueKind),
    Doctype,
    BeforeDoctypeName,
    DoctypeName,
    AfterDoctypeName(usize),
    BogusDoctype,
}

impl Default for TokenState {
    fn default() -> Self {
        TokenState::Data
    }
}

#[derive(Debug, Clone, Copy)]
#[doc(hidden)]
pub enum AttrValueKind {
    Unquoted,
    SingleQuoted,
    DoubleQuoted,
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[doc(hidden)]
pub enum DeclQuote {
    SingleQuoted,
    DoubleQuoted,
}

#[derive(Debug, Clone, Copy)]
#[doc(hidden)]
pub enum XmlDeclQuoteKind {
    SingleQuoted,
    DoubleQuoted,
}

#[derive(Debug, Clone, Copy)]
pub enum DoctypeKind {
    Public,
    System,
}
