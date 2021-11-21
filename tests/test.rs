extern crate xml5;

use xml5::{Tokenizer, TokenResult, Event};

#[test]
fn test_xml() {
    let src = "<xml>".as_bytes();
    let mut reader = Tokenizer::from_reader(src);
    let mut buf = Vec::new();

    match reader.read_event(&mut buf) {
        TokenResult { event: Event::Text(e), .. } => {
            println!("{}", e.to_string());
        }
        _ => {},
    }
}