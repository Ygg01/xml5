use std::io;
use std::io::{BufRead, Error, Read};
use crate::errors::{Xml5Error, Xml5Result};
use crate::tokenizer::reader::FastRead::{InterNeedle, Char};

#[inline(always)]
pub(crate) fn is_whitespace(b: u8) -> bool {
    match b {
        b' ' | b'\r' | b'\n' | b'\t' => true,
        _ => false,
    }
}

pub(crate) trait BufferedInput<'r, 'i, B>
    where
        Self: 'i,
{
    fn peek_byte(&mut self) -> Xml5Result<Option<u8>>;
}

impl<'b, 'i, R: BufRead + 'i> BufferedInput<'b, 'i, &'b mut Vec<u8>> for R
{
    fn peek_byte(&mut self) -> Xml5Result<Option<u8>> {
        loop {
            break match self.fill_buf() {
                Ok(n) if n.is_empty() => Ok(None),
                Ok(n) => Ok(Some(n[0])),
                Err(ref e) if e.kind() == io::ErrorKind::Interrupted => continue,
                Err(e) => break Err(Xml5Error::Io(e.to_string())),
            };
        }
    }
}


#[inline]
pub(crate) fn fast_find(needle: &[u8], haystack: &[u8]) -> Option<usize> {
    #[cfg(feature = "jetscii")]
    {
        debug_assert!(needle.len() <= 16);
        let mut needle_arr = [0; 16];
        needle_arr[..needle.len()].copy_from_slice(needle);
        jetscii::Bytes::new(needle_arr, needle.len() as i32, |b| needle.contains(&b)).find(haystack)
    }

    #[cfg(not(feature = "jetscii"))]
    {
        haystack.iter().position(|b| needle.contains(b))
    }
}

#[derive(PartialEq, Debug)]
pub(crate) enum FastRead {
    Char(u8),
    InterNeedle(Vec<u8>),
    EOF,
}

impl FastRead {
    pub(crate) fn is_ok(&self) -> bool {
        match self {
            Char(_) | InterNeedle(_) => true,
            _ => false
        }
    }
}
