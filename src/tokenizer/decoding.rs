use std::io::BufRead;
use std::str::from_utf8;

#[cfg(feature = "encoding_rs")]
use encoding_rs::Encoding;

use crate::errors::{Xml5Error, Xml5Result};
use crate::tokenizer::emitter::Emitter;
use crate::tokenizer::Tokenizer;

#[cfg(feature = "encoding")]
pub struct Decoder {
    encoding: &'static Encoding,
}

#[cfg(not(feature = "encoding"))]
pub struct Decoder;

impl<'a, R: BufRead, E:Emitter> Tokenizer<R, E> {
    #[cfg(feature = "encoding")]
    pub fn decoder(&self) -> Decoder {
        Decoder {
            encoding: self.encoding
        }
    }

    #[cfg(not(feature = "encoding"))]
    pub fn decoder(&self) -> Decoder {
        Decoder
    }

    /// Decodes a slice using without BOM (Byte order mark) the encoding specified in the XML declaration.
    ///
    /// Decode `bytes` without BOM and with malformed sequences replaced with the
    /// `U+FFFD REPLACEMENT CHARACTER`.
    ///
    /// If no encoding is specified, defaults to UTF-8.
    #[inline]
    #[cfg(feature = "encoding")]
    pub fn decode_without_bom<'b, 'c>(&'b mut self, mut bytes: &'c [u8]) -> Cow<'c, str> {
        if self.is_encoding_set {
            return self.encoding.decode_with_bom_removal(bytes).0;
        }
        if bytes.starts_with(b"\xEF\xBB\xBF") {
            self.is_encoding_set = true;
            bytes = &bytes[3..];
        } else if bytes.starts_with(b"\xFF\xFE") {
            self.is_encoding_set = true;
            self.encoding = encoding_rs::UTF_16LE;
            bytes = &bytes[2..];
        } else if bytes.starts_with(b"\xFE\xFF") {
            self.is_encoding_set = true;
            self.encoding = encoding_rs::UTF_16BE;
            bytes = &bytes[3..];
        };
        self.encoding.decode_without_bom_handling(bytes).0
    }

    /// Decodes a UTF8 slice without BOM (Byte order mark) regardless of XML declaration.
    ///
    /// Decode `bytes` without BOM and with malformed sequences replaced with the
    /// `U+FFFD REPLACEMENT CHARACTER`.
    ///
    /// # Note
    ///
    /// If you instead want to use XML declared encoding, use the `encoding` feature
    #[inline]
    #[cfg(not(feature = "encoding"))]
    pub fn decode_without_bom<'c>(&self, bytes: &'c [u8]) -> Xml5Result<&'c str> {
        if bytes.starts_with(b"\xEF\xBB\xBF") {
            from_utf8(&bytes[3..]).map_err(Xml5Error::Utf8)
        } else {
            from_utf8(bytes).map_err(Xml5Error::Utf8)
        }
    }
}