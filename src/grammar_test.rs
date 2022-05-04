// Note this useful idiom: importing names from outer (for mod tests) scope.
use super::*;
use pest::iterators::Pair;
use juniper::parser::Parser;
use std::collections::HashMap;

#[test]
fn test_grammar_simple1() {
    let input = r#"parent of "func1""#;
    let parser = parser::parser{ map: HashMap::new() };
    let pair: Pair<parser::Rule> = parser.parse_grammar(input.to_string());
    let mut i = 0;
    for inner_pair in pair.into_inner() {
        match inner_pair.as_rule() {
            parser::Rule::command_type => {
                i+=1;
                assert_eq!(inner_pair.as_str(), "parent");
            }
            parser::Rule::function_name => {
                i+=1;
                assert_eq!(inner_pair.as_str(), r#""func1""#);
            }
            _ => {}
        }
    }
    assert_eq!(i, 2);
}

#[test]
fn test_grammar_simple2() {
    let input = r#"child of "func2""#;
    let parser = parser::parser{ map: HashMap::new() };
    let pair: Pair<parser::Rule> = parser.parse_grammar(input.to_string());
    let mut i = 0;
    for inner_pair in pair.into_inner() {
        match inner_pair.as_rule() {
            parser::Rule::command_type => {
                i+=1;
                assert_eq!(inner_pair.as_str(), "child");
            }
            parser::Rule::function_name => {
                i+=1;
                assert_eq!(inner_pair.as_str(), r#""func2""#);
            }
            _ => {}
        }
    }
    assert_eq!(i, 2);
}

#[test]
fn test_grammar1() {
    let input = r#"parent of "func1" where file="123""#;
    let parser = parser::parser{ map: HashMap::new() };
    let pair: Pair<parser::Rule> = parser.parse_grammar(input.to_string());
    let mut i = 0;
    for inner_pair in pair.into_inner() {
        match inner_pair.as_rule() {
            parser::Rule::command_type => {
                i+=1;
                assert_eq!(inner_pair.as_str(), "parent");
            }
            parser::Rule::function_name => {
                i+=1;
                assert_eq!(inner_pair.as_str(), r#""func1""#);
            }
            parser::Rule::where_filter => {
                i+=1;
                assert_eq!(inner_pair.as_str(), r#" where file="123""#);
            }
            _ => {}
        }
    }
    assert_eq!(i, 3);
}

#[test]
fn test_grammar2() {
    let input = r#"parent of "func2" as "function_x""#;
    let parser = parser::parser{ map: HashMap::new() };
    let pair: Pair<parser::Rule> = parser.parse_grammar(input.to_string());
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
    let parser = parser::parser{ map: HashMap::new() };
    let pair: Pair<parser::Rule> = parser.parse_grammar(input.to_string());
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
}