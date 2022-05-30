// Note this useful idiom: importing names from outer (for mod tests) scope.
use super::*;

#[test]
fn test_ast_parser() {
    let input = r#"{@foo}"#;
    let pair = super::parse_ast(input);
    assert!(pair.is_ok());
    println!("{:?}", pair);
}
