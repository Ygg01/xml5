extern crate xml5;

use xml5::Token::StartTag;
use xml5::{Token, Tokenizer};

#[test]
fn test_xml() {
    let src = "<xml>".as_bytes();
    let mut buf = Vec::new();
    let mut reader = Tokenizer::from_reader(src, &mut buf);

    for token in reader {
        match token {
            StartTag(start_tag) => {}
            _ => {}
        }
    }
}
