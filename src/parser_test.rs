use super::*;
use std::collections::HashMap;

#[test]
fn test_parser_simple1() {
    let input = r#"parent of "func1""#;
    let mut parser = parser::parser{ map: HashMap::new() };
    parser.parse(input.to_string());

}