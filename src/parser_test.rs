use super::*;
use std::collections::{HashMap, HashSet};

#[test]
fn test_parser_simple1() {
    let input = r#"{@func}"#;
    let mut parser = parser::parser::new("/Users/hannes.boerner/Downloads/criu-criu-dev".to_string());
    let parser_output = HashSet::from(["parent1".to_string(), "parent2".to_string()]);
    assert_eq!(parser.parse(input), parser_output);
    let graph_output = HashSet::from([("parent1".to_string(), "func".to_string()), ("parent2".to_string(), "func".to_string())]);
    assert_eq!(parser.graph, graph_output);
}

#[test]
fn test_parser_simple2() {
    let input = r#"@func {}"#;
    let mut parser = parser::parser::new("/Users/hannes.boerner/Downloads/criu-criu-dev".to_string());
    let parser_output = HashSet::from(["func".to_string()]);
    assert_eq!(parser.parse(input), parser_output);
    let graph_output = HashSet::from([("func".to_string(), "child1".to_string()), ("func".to_string(), "child2".to_string())]);
    assert_eq!(parser.graph, graph_output);
}

#[test]
fn test_parser() {
    let input = r#"{{@func}}"#;
    let mut parser = parser::parser::new("/Users/hannes.boerner/Downloads/criu-criu-dev".to_string());
    let parser_output = HashSet::from(["parent1".to_string(), "parent2".to_string()]);
    assert_eq!(parser.parse(input), parser_output);
    let graph_output = HashSet::from([
        ("parent1".to_string(), "func".to_string()),
        ("parent1".to_string(), "parent1".to_string()),
        ("parent1".to_string(), "parent2".to_string()),
        ("parent2".to_string(), "func".to_string()),
        ("parent2".to_string(), "parent1".to_string()),
        ("parent2".to_string(), "parent2".to_string())]);
    assert_eq!(parser.graph, graph_output);
    println!("{}", parser.graph_to_DOT());
    parser.graph_to_file("./graph.svg".to_string());
}