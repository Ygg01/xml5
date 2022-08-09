use crate::errors::{Xml5Error, Xml5Result};
use std::io;
use std::io::BufRead;

pub(crate) trait BufferedInput<'r, 'i, B>
where
    Self: 'i,
{
    fn peek_byte(&mut self) -> Xml5Result<Option<u8>>;
}

impl<'b, 'i, R: BufRead + 'i> BufferedInput<'b, 'i, &'b mut Vec<u8>> for R {
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
    InterNeedle(usize, usize),
    EOF,
}
