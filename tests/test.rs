extern crate xml5;

use xml5::Tokenizer;

#[test]
fn test_end_tag() {
    let tokenizer = Tokenizer::new();
    let mut iter = tokenizer.from_str_reader("</xml>");
    let next = iter.next();
    assert_eq!("xml", next.unwrap());
}

#[test]
fn test_pi() {
    let tokenizer = Tokenizer::new();
    let mut iter = tokenizer.from_str_reader("<?xml version=\"0.1\"?>");
    let next = iter.next();
    assert_eq!("xml", next.unwrap());
}
