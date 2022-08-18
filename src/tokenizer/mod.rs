use std::borrow::Cow;

use crate::errors::Xml5Error;
use crate::tokenizer::emitter::SpanTokens::EndTag;
use crate::tokenizer::emitter::{Emitter, SpanTokens, Spans};
#[cfg(feature = "encoding")]
use crate::tokenizer::encoding::EncodingRef;
use crate::tokenizer::reader::{BuffReader, Reader, SliceReader};
use crate::Token;

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
    reader_pos: usize,
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
        let x = loop {
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
        match x {
            EndTag(Some(sp)) => Some(Token::end_tag(self.to_cow(sp))),

            _ => None,
        }
    }
}

impl<'a, E> SliceIterator<'a, E> {
    #[inline(always)]
    fn to_cow(&self, span: Spans) -> Cow<'a, [u8]> {
        match span {
            Spans::Span(range) => Cow::Borrowed(&self.reader.slice[range]),
            Spans::Characters(vc) => Cow::Owned(vc),
        }
    }
}

pub struct BufIterator<'a, R, E> {
    state: Tokenizer,
    reader: BuffReader<'a, R>,
    emitter: E,
}

// impl<'a, R, E> Iterator for BufIterator<'a, R, E>
// where
//     R: BufRead,
//     E: Emitter<OutToken = Token<'a>>,
// {
//     type Item = Token<'a>;
//
//     fn next(&mut self) -> Option<Token<'a>> {
//         loop {
//             if let Some(token) = self.emitter.pop_token() {
//                 break Some(token);
//             } else if !self.state.eof {
//                 match self.state.next_state(&mut self.reader, &mut self.emitter) {
//                     Control::Continue => (),
//                     Control::Eof => {
//                         self.state.eof = true;
//                         self.emitter.emit_eof();
//                     }
//                     Control::Err(e) => break Some(Token::Error(e)),
//                 }
//             } else {
//                 break None;
//             }
//         }
//     }
// }

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
    AfterDoctypeName,
    AfterDoctypeKeyword(DoctypeKind),
    BeforeDoctypeIdentifier(DoctypeKind),
    DoctypeIdentifierDoubleQuoted(DoctypeKind),
    DoctypeIdentifierSingleQuoted(DoctypeKind),
    AfterDoctypeIdentifier(DoctypeKind),
    BetweenDoctypePublicAndSystemIdentifiers,
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

#[derive(Debug, Clone, Copy)]
pub enum DoctypeKind {
    Public,
    System,
}
