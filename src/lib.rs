#[cfg(feature = "encoding_rs")]
extern crate encoding_rs;

mod tokenizer;
mod errors;
mod events;

pub use tokenizer::{Tokenizer};
pub use crate::events::Event;