use std::io::BufRead;

use crate::{Token, Tokenizer};
use crate::errors::{Xml5Error, Xml5Result};
use crate::tokenizer::emitter::{DefaultEmitter, Emitter};
use crate::tokenizer::reader::FastRead::{InterNeedle, Needle};
use crate::tokenizer::reader::Reader;
use crate::tokenizer::TokenState;


impl<R: BufRead> Tokenizer<R> {
}