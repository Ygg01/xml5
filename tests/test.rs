extern crate xml5;

use std::str;

use xml5::Tokenizer;

#[test]
fn test_mini_part() {
    let tokenizer = Tokenizer::new();
    let mut iter = tokenizer.from_str_reader("<xml>test</xml>");
    let next = iter.next();
    assert_eq!(
        "xml",
        str::from_utf8(next.unwrap().get_name().unwrap()).unwrap()
    );
    let next = iter.next();
    assert_eq!(
        "test",
        str::from_utf8(next.unwrap().get_text().unwrap()).unwrap()
    );
    let next = iter.next();
    assert_eq!(
        "xml",
        str::from_utf8(next.unwrap().get_name().unwrap()).unwrap()
    );
}

#[test]
fn test_pi() {
    let tokenizer = Tokenizer::new();
    let mut iter = tokenizer.from_str_reader("<?target data?>");
    let next = iter.next();
    assert_eq!(
        "target",
        str::from_utf8(next.as_ref().unwrap().get_target().unwrap()).unwrap()
    );
    assert_eq!(
        "data",
        str::from_utf8(next.as_ref().unwrap().get_data().unwrap()).unwrap()
    );
}

#[test]
fn test_decl() {
    let tokenizer = Tokenizer::new();
    let mut iter = tokenizer.from_str_reader("<?xml encoding='utf8' ?>");
    let next = iter.next();
    assert_eq!(
        "xml test",
        str::from_utf8(next.as_ref().unwrap().get_declaration().unwrap()).unwrap()
    );
}
