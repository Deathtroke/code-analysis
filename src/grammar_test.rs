// Note this useful idiom: importing names from outer (for mod tests) scope.
use super::*;
use pest::iterators::Pair;
use juniper::parser::Parser;
use std::collections::HashMap;
use regex::Regex;

#[test]
fn test_grammar_simple1() {
    let input = r#"{@foo}"#;
    let pair: Pair<parser::Rule> = parser::parse_grammar(input).unwrap().next().unwrap();
    let mut i = 0;
    for inner_pair in pair.into_inner() {
        match inner_pair.as_rule() {
            parser::Rule::statement => {
                for statement in inner_pair.into_inner() {
                    match statement.as_rule() {
                        parser::Rule::scope => {
                            for scope in statement.into_inner() {
                                match scope.as_rule() {
                                    parser::Rule::statements => {
                                        for statements2 in scope.into_inner() {
                                            match statements2.as_rule() {
                                                parser::Rule::statement => {
                                                    for statement2 in statements2.into_inner() {
                                                        match statement2.as_rule() {
                                                            parser::Rule::verb => {
                                                                i += 1;
                                                                assert_eq!(statement2.as_str(), "@foo");
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
    assert_eq!(i, 1);
}

#[test]
fn test_grammar_simple2() {
    let input = r#"@foo{}"#;
    let pair: Pair<parser::Rule> = parser::parse_grammar(input).unwrap().next().unwrap();
    let mut i = 0;
    for inner_pair in pair.into_inner() {
        match inner_pair.as_rule() {
            parser::Rule::statement => {
                for statement in inner_pair.into_inner() {
                    match statement.as_rule() {
                        parser::Rule::scope => {
                            i += 1;
                            assert_eq!(statement.as_str(), "{}");
                        }
                        parser::Rule::verb => {
                            i += 1;
                            assert_eq!(statement.as_str(), "@foo");
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
    let input = r#"
    @foo{
      @bar{}
    }"#;
    let pair: Pair<parser::Rule> = parser::parse_grammar(input).unwrap().next().unwrap();
    let mut i = 0;
    for inner_pair in pair.into_inner() {
        match inner_pair.as_rule() {
            parser::Rule::statement => {
                for statement in inner_pair.into_inner() {
                    match statement.as_rule() {
                        parser::Rule::verb => {
                            i += 1;
                            assert_eq!(statement.as_str(), "@foo");
                        }
                        parser::Rule::scope => {
                            for scope in statement.into_inner() {
                                match scope.as_rule() {
                                    parser::Rule::statements => {
                                        for statements2 in scope.into_inner() {
                                            match statements2.as_rule() {
                                                parser::Rule::statement => {
                                                    for statement2 in statements2.into_inner() {
                                                        match statement2.as_rule() {
                                                            parser::Rule::verb => {
                                                                i += 1;
                                                                assert_eq!(statement2.as_str(), "@bar");
                                                            }
                                                            parser::Rule::scope => {
                                                                i += 1;
                                                                assert_eq!(statement2.as_str(), "{}");
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

#[test]
fn test_grammar2() {
    let input = r#"@foo @bar {@tar}"#;
    let pair: Pair<parser::Rule> = parser::parse_grammar(input).unwrap().next().unwrap();
    let mut i = 0;
    for inner_pair in pair.into_inner() {
        match inner_pair.as_rule() {
            parser::Rule::statement => {
                for statement in inner_pair.into_inner() {
                    match statement.as_rule() {
                        parser::Rule::verb => {
                            i += 1;
                            if i == 1 {
                                assert_eq!(statement.as_str(), "@foo");
                            } else {
                                assert_eq!(statement.as_str(), "@bar");
                            }
                        }
                        parser::Rule::scope => {
                            for scope in statement.into_inner() {
                                match scope.as_rule() {
                                    parser::Rule::statements => {
                                        for statements2 in scope.into_inner() {
                                            match statements2.as_rule() {
                                                parser::Rule::statement => {
                                                    for statement2 in statements2.into_inner() {
                                                        match statement2.as_rule() {
                                                            parser::Rule::verb => {
                                                                i += 1;
                                                                assert_eq!(statement2.as_str(), "@tar");
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

#[test]
fn test_grammar_complex1() {
    let input = r#"
    @foo @bar {
      @foo @bar {
      };
      @foo @tar;
    };
    @foo"#;
    let pair: Pair<parser::Rule> = parser::parse_grammar(input).unwrap().next().unwrap();
    let mut i = 0;
    for inner_pair in pair.into_inner() {
        match inner_pair.as_rule() {
            parser::Rule::statement => {
                for statement in inner_pair.into_inner() {
                    match statement.as_rule() {
                        parser::Rule::verb => {
                            i += 1;
                            match i {
                                1 => {assert_eq!(statement.as_str(), "@foo");}
                                2 => {assert_eq!(statement.as_str(), "@bar");}
                                _ => {assert_eq!(statement.as_str(), "@foo");}
                            }
                        }
                        parser::Rule::scope => {
                            for scope in statement.into_inner() {
                                match scope.as_rule() {
                                    parser::Rule::statements => {
                                        let mut verb_nr = 0;
                                        for statements2 in scope.into_inner() {
                                            match statements2.as_rule() {
                                                parser::Rule::statement => {
                                                    for statement2 in statements2.into_inner() {
                                                        match statement2.as_rule() {
                                                            parser::Rule::verb => {
                                                                i += 1;
                                                                verb_nr += 1;
                                                                //println!("{} -> <{}>", verb_nr, statement2.as_str());
                                                                match verb_nr {
                                                                    1 => {assert_eq!(statement2.as_str(), "@foo");}
                                                                    2 => {assert_eq!(statement2.as_str(), "@bar");}
                                                                    //next statement
                                                                    3 => {assert_eq!(statement2.as_str(), "@foo");}
                                                                    4 => {assert_eq!(statement2.as_str(), "@tar");}
                                                                    _ => {}
                                                                }
                                                            }
                                                            parser::Rule::scope => {
                                                                i += 1;
                                                                let re = Regex::new(r"\{([ \n])*}").unwrap();
                                                                assert!(re.is_match(statement2.as_str()));
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

    assert_eq!(i, 8);
}