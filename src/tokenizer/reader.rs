use std::borrow::Cow;
use std::io;
use std::io::{BufRead, Read};

use crate::errors::Xml5Error::UnexpectedEof;
use crate::errors::{Xml5Error, Xml5Result};
use crate::tokenizer::emitter::Spans;

pub(crate) trait Reader<'r> {
    fn peek_byte(&mut self) -> Xml5Result<Option<u8>>;
    fn consume_bytes(&mut self, amount: usize);
    fn slice_bytes(&self, start: usize, end: usize) -> &[u8];
    fn append_curr_char(&mut self) -> usize;

    fn try_read_slice(&mut self, needle: &str, case_sensitive: bool) -> bool;
    #[inline(always)]
    fn try_read_slice_exact(&mut self, needle: &str) -> bool {
        self.try_read_slice(needle, true)
    }
    fn read_fast_until(&mut self, needle: &[u8]) -> FastRead;
}

pub struct BuffReader<'a, S> {
    pub source: S,
    pub buffer: &'a mut Vec<u8>,
}

impl<'a, R: BufRead> BuffReader<'a, R> {
    pub(crate) fn from_str<'s>(source: R, buffer: &'a mut Vec<u8>) -> BuffReader<'a, R> {
        Self { source, buffer }
    }
}

impl<'b, B> Reader<'b> for BuffReader<'b, B>
where
    B: BufRead,
{
    fn peek_byte(&mut self) -> Xml5Result<Option<u8>> {
        loop {
            break match self.source.fill_buf() {
                Ok(n) if n.is_empty() => Ok(None),
                Ok(n) => Ok(Some(n[0])),
                Err(ref e) if e.kind() == io::ErrorKind::Interrupted => continue,
                Err(e) => break Err(Xml5Error::Io(e.to_string())),
            };
        }
    }

    fn consume_bytes(&mut self, amount: usize) {
        self.source.consume(amount);
    }

    fn slice_bytes(&self, start: usize, end: usize) -> &[u8] {
        &self.buffer[start..end]
    }

    fn append_curr_char(&mut self) -> usize {
        if let Ok(Some(x)) = self.peek_byte() {
            self.buffer.push(x);
            return self.buffer.len() - 1;
        }
        panic!("This method shouldn't be called if there isn't a reader.peek_byte")
    }

    fn try_read_slice(&mut self, needle: &str, case_sensitive: bool) -> bool {
        let mut buff: Vec<u8> = Vec::new();
        while buff.is_empty() {
            match self.source.fill_buf() {
                Ok(n) if n.is_empty() => return false,
                Ok(n) => buff.extend_from_slice(n),
                Err(ref e) if e.kind() == io::ErrorKind::Interrupted => continue,
                Err(_) => return false,
            };
        }

        if buff.len() < needle.len() {
            return false;
        }

        let read = if case_sensitive {
            buff[0..needle.len()].starts_with(needle.as_bytes())
        } else {
            needle.as_bytes().iter().enumerate().all(|(offset, char)| {
                buff[offset].to_ascii_lowercase() == char.to_ascii_lowercase()
            })
        };

        if read {
            self.source.consume(needle.len());
        }
        read
    }

    fn read_fast_until(&mut self, needle: &[u8]) -> FastRead {
        loop {
            // fill buffer
            let available = match self.source.fill_buf() {
                Ok(n) if n.is_empty() => return FastRead::EOF,
                Ok(n) => n,
                Err(ref e) if e.kind() == io::ErrorKind::Interrupted => continue,
                Err(_) => return FastRead::EOF,
            };

            let (read, n) = match fast_find(needle, &available[..]) {
                Some(0) => (FastRead::Char(available[0]), 1),
                Some(size) => {
                    let start = self.buffer.len();
                    self.buffer.extend_from_slice(&available[..size]);
                    (FastRead::InterNeedle(start, start + size), size)
                }
                None => (FastRead::EOF, 0),
            };
            self.consume_bytes(n);
            return read;
        }
    }
}

pub struct SliceReader<'a> {
    pub slice: &'a [u8],
    pos: usize,
}

impl<'a> SliceReader<'a> {
    pub(crate) fn from_str(input: &'a str) -> SliceReader<'a> {
        Self {
            slice: input.as_bytes(),
            pos: 0,
        }
    }
}

impl<'r> Reader<'r> for SliceReader<'r> {
    fn peek_byte(&mut self) -> Xml5Result<Option<u8>> {
        match self.slice.get(self.pos) {
            Some(x) => Ok(Some(*x)),
            _ => Err(UnexpectedEof),
        }
    }

    fn consume_bytes(&mut self, amount: usize) {
        self.pos += amount;
    }

    fn slice_bytes(&self, start: usize, end: usize) -> &'r [u8] {
        &self.slice[start..end]
    }

    fn append_curr_char(&mut self) -> usize {
        self.pos
    }

    fn try_read_slice(&mut self, needle: &str, case_sensitive: bool) -> bool {
        if self.slice.len() < needle.len() {
            return false;
        }

        let read = if case_sensitive {
            self.slice[self.pos..self.pos + needle.len()].starts_with(needle.as_bytes())
        } else {
            needle.as_bytes().iter().enumerate().all(|(offset, char)| {
                self.slice[self.pos + offset].to_ascii_lowercase() == char.to_ascii_lowercase()
            })
        };

        if read {
            self.pos += needle.len();
        }
        read
    }

    fn read_fast_until(&mut self, needle: &[u8]) -> FastRead {
        let (read, n) = match fast_find(needle, &self.slice[self.pos..]) {
            Some(0) => (FastRead::Char(self.slice[self.pos]), 1),
            Some(size) => (FastRead::InterNeedle(self.pos, self.pos + size), size),
            None => (FastRead::EOF, 0),
        };
        self.pos += n;
        read
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

trait TestReader<'a>: Reader<'a> {
    fn test_read_fast(&mut self, needle: &str) -> String {
        match self.read_fast_until(needle.as_bytes()) {
            FastRead::Char(chr) => format!("{}", chr as char),
            FastRead::InterNeedle(s, e) => {
                String::from_utf8(self.slice_bytes(s, e).to_vec()).unwrap()
            }
            FastRead::EOF => "".to_string(),
        }
    }
}

impl<'a, T> TestReader<'a> for T where T: Reader<'a> {}

#[allow(unused_macros)]
macro_rules! test_readers {
    (($($e:expr),+ ). $me:ident ($arg:expr) = $eq:expr) => {
        $(
            assert_eq!($eq, $e.$me($arg).as_str());
        )+
    };
    (($($e:expr),+ ). $me:ident $arg:tt) => {
        $(
            assert!($e.$me$arg);
        )+
    };
    (($($e:expr),+ ). $me:ident $arg:tt = false) => {
        $(
            assert!(!$e.$me$arg);
        )+
    };
}

#[test]
pub fn test_read_until() {
    let source = "TestString";
    let mut buf = vec![];
    let mut buff_reader = BuffReader::from_str(source.as_bytes(), &mut buf);
    let mut str_reader = SliceReader::from_str(source);

    test_readers!((buff_reader, str_reader).test_read_fast("r") = "TestSt");
    test_readers!((buff_reader, str_reader).test_read_fast("r") = "r");
    test_readers!((buff_reader, str_reader).test_read_fast("g") = "in");
    test_readers!((buff_reader, str_reader).test_read_fast("g") = "g");
    test_readers!((buff_reader, str_reader).test_read_fast("g") = "");
}

#[test]
fn test_read_until2() {
    let source = "xyz_abc";
    let mut buf = vec![];
    let mut buff_reader = BuffReader::from_str(source.as_bytes(), &mut buf);
    let mut str_reader = SliceReader::from_str(source);

    test_readers!((buff_reader, str_reader).test_read_fast("x") = "x");
    test_readers!((buff_reader, str_reader).test_read_fast("y") = "y");
    test_readers!((buff_reader, str_reader).test_read_fast("z") = "z");
    test_readers!((buff_reader, str_reader).test_read_fast("??") = "");
}

#[test]
fn test_try_read_slice1() {
    let source = "xyz_abc";
    let mut buf = vec![];
    let mut buff_reader = BuffReader::from_str(source.as_bytes(), &mut buf);
    let mut str_reader = SliceReader::from_str(source);

    test_readers!((buff_reader, str_reader).try_read_slice("?A?", true) = false);
    test_readers!((buff_reader, str_reader).try_read_slice("?A?", false) = false);
    test_readers!((buff_reader, str_reader).try_read_slice("xyz", true));
    test_readers!((buff_reader, str_reader).try_read_slice("_AbC", true) = false);
    test_readers!((buff_reader, str_reader).try_read_slice("_AbC", false));
}
