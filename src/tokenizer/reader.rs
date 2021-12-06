use std::io;
use std::io::BufRead;
use crate::errors::{Xml5Error, Xml5Result};
use crate::tokenizer::reader::FastRead::Needle;

pub(crate) trait Reader<'r, 'i, B>
    where
        Self: 'i
{
    fn peek_byte(&mut self) -> Xml5Result<Option<u8>>;
    fn read_fast_until2(&mut self, needle1: u8, needle2: u8) -> FastRead;
}

impl<'r: 'i, 'i, B: BufRead + 'i> Reader<'r, 'i, B> for B {
    fn peek_byte(&mut self) -> Xml5Result<Option<u8>> {
        loop {
            break match self.fill_buf() {
                Ok(n) if n.is_empty() => Ok(None),
                Ok(n) => Ok(Some(n[0])),
                Err(ref e) if e.kind() == io::ErrorKind::Interrupted => continue,
                Err(e) => Err(Xml5Error::Io(e)),
            };
        }
    }

    fn read_fast_until2(
        &mut self,
        needle1: u8,
        needle2: u8,
    ) -> FastRead {
        // If previous memchr was searched until the very needle, needle will be a first element
        match self.peek_byte() {
            Ok(Some(chr)) if chr == needle1 || chr == needle1 => return Needle(chr),
            _ => (),
        };
        let mut read = 0usize;
        let mut done = false;

        let mut buf = vec![];
        while !done {
            let used = {
                let available = match self.fill_buf() {
                    Ok(n) if n.is_empty() => return FastRead::EOF,
                    Ok(n) => n,
                    Err(ref e) if e.kind() == io::ErrorKind::Interrupted => continue,
                    Err(_) => return FastRead::EOF,
                };

                match memchr::memchr2(needle1, needle2, available)
                {
                    // Read until the needle, omitting it
                    Some(i) => {
                        buf.extend_from_slice(&available[..i - 1]);
                        done = true;
                        i
                    }
                    None => {
                        buf.extend_from_slice(available);
                        available.len()
                    }
                }
            };
            self.consume(used);
            read += used;
        }

        if read != 0
        {
            FastRead::InterNeedle(buf)
        } else {
            // we reached the end somehow
            FastRead::Needle(b'\0')
        }
    }
}

pub(crate) enum FastRead {
    Needle(u8),
    InterNeedle(Vec<u8>),
    EOF,
}
