// Temporary disable
#![allow(unused_must_use)]

#[cfg(feature = "encoding_rs")]
extern crate encoding_rs;
extern crate core;

mod encoding;
mod errors;
mod events;
mod tokenizer;

pub use crate::events::Token;
pub use tokenizer::Tokenizer;
