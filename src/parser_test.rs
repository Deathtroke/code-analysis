use super::*;
use std::collections::HashMap;

#[test]
fn test_parser_simple1() {
    let input = r#"parent of "func""#;
    let mut parser = parser::parser{ graph:Vec::new() };
    println!("{:?}", parser.parse(input));
    println!("{:?}",parser.graph);

}