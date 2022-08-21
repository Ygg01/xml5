extern crate xml5;

use xml5::Tokenizer;

#[test]
fn test_end_tag() {
    let tokenizer = Tokenizer::new();
    let mut iter = tokenizer.from_str_reader("</xml>");
    let next = iter.next();
    assert_eq!("xml".as_bytes(), next.unwrap().get_name().unwrap());
}
#[test]
fn test_pi() {
    let tokenizer = Tokenizer::new();
    let mut iter = tokenizer.from_str_reader("<?xml test?>");
    let next = iter.next();
    assert_eq!("xml".as_bytes(), next.unwrap().get_target().unwrap());
}
