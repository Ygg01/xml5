use std::borrow::Cow;
use std::io;
use std::io::BufRead;
use std::ops::Range;
use crate::errors::{Error, Result};
use crate::errors::Error::Eof;

pub(crate) trait Reader<'r, 'i, B>
    where
        Self: 'i
{
    fn read_pos(&mut self, pos: usize) -> Result<Option<u8>>;
    
    fn read_range(&mut self, buf: B, range: Range<usize>) -> Result<Option<&[u8]>>;
}

impl<'b, 'i, R: BufRead + 'i> Reader<'b, 'i, &'b mut Vec<u8>> for R {
    fn read_pos(&mut self, pos: usize) -> Result<Option<u8>> {
        loop {
            break match self.fill_buf() {
                Ok(n) => {
                    if n.is_empty() || pos >= n.len() {
                        Ok(None)
                    } else {
                        Ok(Some(n[pos]))
                    }
                }
                Err(ref e) if e.kind() == io::ErrorKind::Interrupted => continue,
                Err(e) => Err(Error::Io(e)),
            };
        }
    }
    
    fn read_range(&mut self, buf: &'b mut Vec<u8>, range: Range<usize>) -> Result<Option<&[u8]>> {
        let mut done = false;
        while !done {
            let available = match self.fill_buf() {
                Ok(n) if n.is_empty() => break,
                Ok(n) => n,
                Err(ref e) if e.kind() == io::ErrorKind::Interrupted => continue,
                Err(e) => {
                    return Err(Error::Io(e));
                }
            };

            if available.len() >= range.end {
                done = true;
                buf.extend_from_slice(&available[range.start..range.end])
            }
        }

        Ok(None)
    }
}

impl<'a> Reader<'a, 'a, ()> for &'a [u8] {
    fn read_pos(&mut self, pos: usize) -> Result<Option<u8>> {
        if pos >= self.len() {
            Err(Eof)
        } else {
            Ok(Some(self[pos]))
        }
    }

    fn read_range(&mut self, buf: (), range: Range<usize>) -> Result<Option<&[u8]>> {
        if range.end < self.len()  {
            Ok(Some(&self[range]))
        } else {
            Err(Eof)
        }
    }
}