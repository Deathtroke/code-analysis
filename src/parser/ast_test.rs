use pest::error::InputLocation;
// Note this useful idiom: importing names from outer (for mod tests) scope.
use super::*;
use pest::iterators::Pair;
use regex::Regex;

#[test]
fn test_ast_parser() {
    let input = r#"{@foo}"#;
    let pair = parse_ast(input);
    assert!(pair.is_ok());
    println!("{:?}", pair);
}
