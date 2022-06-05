use std::borrow::Borrow;
use pest::error::Error;
// Note this useful idiom: importing names from outer (for mod tests) scope.
use super::*;

#[test]
fn test_ast_parser_successful() {
    let input = r#"{@foo}"#;
    let ast = super::parse_ast(input);
    assert!(ast.is_ok());
    //println!("{:?}", ast.unwrap());
}

#[test]
fn test_ast_parser_simple() {
    let input = r#"{@foo}"#;
    let ast = super::parse_ast(input);
    let mut i = 0;
    match ast.unwrap().last().unwrap().to_owned() {
        AstNode::Print(print) => {
            match *print.clone() {
                AstNode::Statements(statements) => {
                    match statements.last().unwrap().to_owned() {
                        AstNode::Statement { verb, scope } => {
                            assert_eq!(verb.len(), 0);
                            match scope.unwrap().to_owned() {
                                AstNode::Scope(inner_scope) => {
                                    match *inner_scope {
                                        AstNode::Statements(statements) => {
                                            match statements.last().unwrap().to_owned() {
                                                AstNode::Statement { verb, .. } => {
                                                    match verb.last().unwrap().to_owned() {
                                                        AstNode::Verb { ident, .. } => {
                                                            i+=1;
                                                            assert_eq!(format!("{:?}", ident), r#"Ident("foo")"#);
                                                        }
                                                        _=>{}
                                                    }
                                                }
                                                _ => {}
                                            }
                                        }
                                        _ => {}
                                    }
                                }
                                _ => {}
                            }
                        }
                        _ =>{}
                    }
                }
                _ => {}
            }
        }
        _ => {}
    }
    assert_eq!(i, 1);
}

#[test]
fn test_ast_parser_rebuild() {
    let input = r#"@foo{}"#;
    let ast = super::parse_ast(input);
    let inner_statement = parser::AstNode::Statement { verb: vec![], scope: Box::new(None) };
    let inner_statements = parser::AstNode::Statements(vec![inner_statement]);
    let scope = parser::AstNode::Scope(Box::new(inner_statements));
    let verb = parser::AstNode::Verb { ident: Box::new(parser::AstNode::Ident("foo".to_string())), named_parameter: vec![] };
    let statement = parser::AstNode::Statement { verb: vec![verb], scope: Box::new(Some(scope)) };
    let statements = parser::AstNode::Statements(vec![statement]);
    let print = parser::AstNode::Print(Box::new(statements));
    assert_eq!(format!("{:?}",ast.unwrap().last().unwrap().to_owned()),
               format!("{:?}",print));
}