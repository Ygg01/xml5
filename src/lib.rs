// Temporary disable
#![allow(unused_must_use)]

extern crate core;
#[cfg(feature = "encoding_rs")]
extern crate encoding_rs;

pub mod encoding;
mod errors;
mod events;
mod tokenizer;

pub use crate::events::Token;
pub use tokenizer::Tokenizer;
