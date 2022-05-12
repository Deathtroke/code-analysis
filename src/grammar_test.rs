// Note this useful idiom: importing names from outer (for mod tests) scope.
use super::*;
use pest::iterators::Pair;
use juniper::parser::Parser;
use std::collections::HashMap;

#[test]
fn test_grammar_simple1() {
    let input = r#"@any{foo;};"#;
    let pair: Pair<parser::Rule> = parser::parse_grammar(input);
    let mut i = 0;
    for inner_pair in pair.into_inner() {
        match inner_pair.as_rule() {
            parser::Rule::requests => {
                for requests in inner_pair.into_inner() {
                    match requests.as_rule() {
                        parser::Rule::request_expr => {
                            for request_expr in requests.into_inner() {
                                match request_expr.as_rule() {
                                    parser::Rule::function_filter => {
                                        i += 1;
                                        assert_eq!(request_expr.as_str(), "@any");
                                    }
                                    parser::Rule::child_expr => {
                                        for requests2 in request_expr.into_inner() {
                                            match requests2.as_rule() {
                                                parser::Rule::requests => {
                                                    for request_expr2 in requests2.into_inner() {
                                                        match request_expr2.as_rule() {
                                                            parser::Rule::request_expr => {
                                                                i += 1;
                                                                assert_eq!(request_expr2.as_str(), "foo");
                                                            }
                                                            _ => {}
                                                        }
                                                    }
                                                }
                                                _ => {}
                                            }
                                        }
                                    }
                                    _ => {}
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }
    assert_eq!(i, 2);
}

#[test]
fn test_grammar_simple2() {
    let input = r#"foo{}"#;
    let pair: Pair<parser::Rule> = parser::parse_grammar(input);
    let mut i = 0;
    for inner_pair in pair.into_inner() {
        match inner_pair.as_rule() {
            parser::Rule::requests => {
                for requests in inner_pair.into_inner() {
                    match requests.as_rule() {
                        parser::Rule::request_expr => {
                            for request_expr in requests.into_inner() {
                                match request_expr.as_rule() {
                                    parser::Rule::function_filter => {
                                        i += 1;
                                        assert_eq!(request_expr.as_str(), "foo");
                                    }
                                    parser::Rule::child_expr => {
                                        i += 1;
                                        assert_eq!(request_expr.as_str(), "{}");
                                    }
                                    _ => {}
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }
    assert_eq!(i, 2);
}

#[test]
fn test_grammar1() {
    let input =
        r#"foo {
  bar {
  };
};"#;
    let pair: Pair<parser::Rule> = parser::parse_grammar(input);
    let mut i = 0;
    for inner_pair in pair.into_inner() {
        match inner_pair.as_rule() {
            parser::Rule::requests => {
                for requests in inner_pair.into_inner() {
                    match requests.as_rule() {
                        parser::Rule::request_expr => {
                            for request_expr in requests.into_inner() {
                                match request_expr.as_rule() {
                                    parser::Rule::function_filter => {
                                        i += 1;
                                        assert_eq!(request_expr.as_str(), "foo ");
                                    }
                                    parser::Rule::child_expr => {
                                        for child_expr in request_expr.into_inner() {
                                            match child_expr.as_rule() {
                                                parser::Rule::requests => {
                                                    for request2 in child_expr.into_inner() {
                                                        match request2.as_rule() {
                                                            parser::Rule::request_expr => {
                                                                for request_expr2 in request2.into_inner() {
                                                                    match request_expr2.as_rule() {
                                                                        parser::Rule::function_filter => {
                                                                            i += 1;
                                                                            assert_eq!(request_expr2.as_str(), "bar ");
                                                                        }
                                                                        parser::Rule::child_expr => {
                                                                            i += 1;
                                                                            assert_eq!(request_expr2.as_str(), "{\n  }");
                                                                        }
                                                                        _ => {}
                                                                    }
                                                                }
                                                            }
                                                            _ => {}
                                                        }
                                                    }
                                                }
                                                _ => {}
                                            }
                                        }
                                    }
                                    _ => {}
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }
    assert_eq!(i, 3);
}
/*
#[test]
fn test_grammar2() {
    let input = r#"parent of "func2" as "function_x""#;
    let pair: Pair<parser::Rule> = parser::parse_grammar(input);
    let mut i = 0;
    for inner_pair in pair.into_inner() {
        match inner_pair.as_rule() {
            parser::Rule::command_type => {
                i+=1;
                assert_eq!(inner_pair.as_str(), "parent");
            }
            parser::Rule::function_name => {
                i+=1;
                assert_eq!(inner_pair.as_str(), r#""func2""#);
            }
            parser::Rule::overwrite => {
                i+=1;
                assert_eq!(inner_pair.as_str(), r#" as "function_x""#);
            }
            _ => {}
        }
    }
    assert_eq!(i, 3);
}

#[test]
fn test_grammar_complex1() {
    let input = r#"parent of {parent of "func1" where @new:filter(file = "abc")} where @filter"#;
    let pair: Pair<parser::Rule> = parser::parse_grammar(input);
    let mut i = 0;
    for inner_pair in pair.into_inner() {
        match inner_pair.as_rule() {
            parser::Rule::command_type => {
                i+=1;
                assert_eq!(inner_pair.as_str(), "parent");
            }

            parser::Rule::functions => {
                i+=1;
                assert_eq!(inner_pair.as_str(), r#"{parent of "func1" where @new:filter(file = "abc")}"#);

                for functions in inner_pair.into_inner() {
                    match functions.as_rule() {
                        parser::Rule::extra_command => {
                            //println!("{:?}", functions);

                            for extra_command_pair in functions.into_inner() {
                                match extra_command_pair.as_rule() {
                                    parser::Rule::command => {
                                        //println!("{:?}", extra_command_pair);

                                        for command_pair in extra_command_pair.into_inner() {
                                            match command_pair.as_rule() {
                                                parser::Rule::function_name => {
                                                    i+=1;
                                                    assert_eq!(command_pair.as_str(), r#""func1""#);
                                                }
                                                parser::Rule::where_filter => {
                                                    //println!("{:?}", command_pair);

                                                    for where_filter_pair in command_pair.into_inner() {
                                                        match where_filter_pair.as_rule() {
                                                            parser::Rule::params => {
                                                                //println!("{:?}", where_filter_pair);

                                                                for params_pair in where_filter_pair.into_inner() {
                                                                    match params_pair.as_rule() {
                                                                        parser::Rule::macro_filter => {
                                                                            i+=1;
                                                                            assert_eq!(params_pair.as_str(), r#"@new:filter(file = "abc")"#);
                                                                        }
                                                                        _ => {}
                                                                    }
                                                                }
                                                            }
                                                            _ => {}
                                                        }
                                                    }
                                                }
                                                _ => {}
                                            }
                                        }
                                    }
                                    _ => {}
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }


            parser::Rule::where_filter => {
                i+=1;
                assert_eq!(inner_pair.as_str(), " where @filter");
            }
            _ => {}
        }
    }
    assert_eq!(i, 5);
}*/