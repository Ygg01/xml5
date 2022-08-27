// Temporary disable
#![allow(unused_must_use)]

extern crate core;
#[cfg(feature = "encoding_rs")]
extern crate encoding_rs;

pub use tokenizer::Tokenizer;

pub use crate::events::Token;

pub mod encoding;
mod errors;
mod events;
mod tokenizer;
