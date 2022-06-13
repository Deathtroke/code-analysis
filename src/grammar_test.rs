use pest::error::InputLocation;
// Note this useful idiom: importing names from outer (for mod tests) scope.
use super::*;
use pest::iterators::Pair;
use regex::Regex;


#[test]
fn test_grammar_simple1() {
    let input = r#"{@foo}"#;
    let pair: Pair<ast_generator::Rule> = ast_generator::parse_grammar(input).unwrap().next().unwrap();
    let mut i = 0;
    let inner_pair =  pair.into_inner().last().unwrap();
    match inner_pair.as_rule() {
        ast_generator::Rule::statement => {
            let statement = inner_pair.into_inner().last().unwrap();
            match statement.as_rule() {
                ast_generator::Rule::scope => {
                    let scope = statement.into_inner().last().unwrap();
                    match scope.as_rule() {
                        ast_generator::Rule::statements => {
                            let statements2 = scope.into_inner().last().unwrap();
                            match statements2.as_rule() {
                                ast_generator::Rule::statement => {
                                    let statement2 = statements2.into_inner().last().unwrap();
                                    match statement2.as_rule() {
                                        ast_generator::Rule::verb => {
                                            i += 1;
                                            assert_eq!(
                                                statement2.as_str(),
                                                "@foo"
                                            );
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
                _ => {}
            }
        }
        _ => {}
    }
    assert_eq!(i, 1);
}

#[test]
fn test_grammar_simple2() {
    let input = r#"@foo{}"#;
    let pair: Pair<ast_generator::Rule> = ast_generator::parse_grammar(input).unwrap().next().unwrap();
    let mut i = 0;
    for inner_pair in pair.into_inner() {
        match inner_pair.as_rule() {
            ast_generator::Rule::statement => {
                for statement in inner_pair.into_inner() {
                    match statement.as_rule() {
                        ast_generator::Rule::scope => {
                            i += 1;
                            assert_eq!(statement.as_str(), "{}");
                        }
                        ast_generator::Rule::verb => {
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
    let pair: Pair<ast_generator::Rule> = ast_generator::parse_grammar(input).unwrap().next().unwrap();
    let mut i = 0;
    let inner_pair = pair.clone().into_inner().last().unwrap();
    let mut statements2 = pair;
    match inner_pair.as_rule() {
        ast_generator::Rule::statement => {
            for statement in inner_pair.into_inner() {
                match statement.as_rule() {
                    ast_generator::Rule::verb => {
                        i += 1;
                        assert_eq!(statement.as_str(), "@foo");
                    }
                    ast_generator::Rule::scope => {
                        let scope = statement.into_inner().last().unwrap();
                        match scope.as_rule() {
                            ast_generator::Rule::statements => {
                                statements2 = scope.into_inner().nth(0).unwrap();
                            }
                            _ => {}
                        }
                    }
                    _ => {}
                }
            }
        }
        _ => {}
    }

    match statements2.as_rule() {
        ast_generator::Rule::statement => {
            for statement2 in statements2.into_inner() {
                match statement2.as_rule() {
                    ast_generator::Rule::verb => {
                        i += 1;
                        assert_eq!(
                            statement2.as_str(),
                            "@bar"
                        );
                    }
                    ast_generator::Rule::scope => {
                        i += 1;
                        assert_eq!(
                            statement2.as_str(),
                            "{}"
                        );
                    }
                    _ => {}
                }
            }
        }
        _ => {}
    }

    assert_eq!(i, 3);
}

#[test]
fn test_grammar2() {
    let input = r#"@foo @bar {@tar}"#;
    let pair: Pair<ast_generator::Rule> = ast_generator::parse_grammar(input).unwrap().next().unwrap();
    let mut i = 0;
    let inner_pair = pair.clone().into_inner().last().unwrap();
    let mut statements2 = pair;
    match inner_pair.as_rule() {
        ast_generator::Rule::statement => {
            for statement in inner_pair.into_inner() {
                match statement.as_rule() {
                    ast_generator::Rule::verb => {
                        i += 1;
                        if i == 1 {
                            assert_eq!(statement.as_str(), "@foo");
                        } else {
                            assert_eq!(statement.as_str(), "@bar");
                        }
                    }
                    ast_generator::Rule::scope => {
                        let scope = statement.into_inner().last().unwrap();
                        match scope.as_rule() {
                            ast_generator::Rule::statements => {
                                statements2 = scope.into_inner().last().unwrap();
                            }
                            _ => {}
                        }
                    }
                    _ => {}
                }
            }
        }
        _ => {}
    }

    match statements2.as_rule() {
        ast_generator::Rule::statement => {
            for statement2 in statements2.into_inner() {
                match statement2.as_rule() {
                    ast_generator::Rule::verb => {
                        i += 1;
                        assert_eq!(
                            statement2.as_str(),
                            "@tar"
                        );
                    }
                    _ => {}
                }
            }
        }
        _ => {}
    }

    assert_eq!(i, 3);
}

#[test]
fn test_wrong_space() {
    let input = r#"{@ tar}"#;
    let result = ast_generator::parse_grammar(input);
    assert!(result.is_err());
    assert_eq!(result.err().unwrap().location, InputLocation::Pos(2));
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
    let pair: Pair<ast_generator::Rule> = ast_generator::parse_grammar(input).unwrap().next().unwrap();
    let mut i = 0;
    let mut statements2 = pair.clone().into_inner();
    for inner_pair in pair.into_inner() {
        match inner_pair.as_rule() {
            ast_generator::Rule::statement => {
                for statement in inner_pair.into_inner() {
                    match statement.as_rule() {
                        ast_generator::Rule::verb => {
                            i += 1;
                            match i {
                                1 => {
                                    assert_eq!(statement.as_str(), "@foo");
                                }
                                2 => {
                                    assert_eq!(statement.as_str(), "@bar");
                                }
                                _ => {
                                    assert_eq!(statement.as_str(), "@foo");
                                }
                            }
                        }
                        ast_generator::Rule::scope => {
                            for scope in statement.into_inner() {
                                match scope.as_rule() {
                                    ast_generator::Rule::statements => {
                                        statements2 = scope.into_inner();
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

    let mut verb_nr = 0;
    for statements2_inner in statements2 {
        match statements2_inner.as_rule() {
            ast_generator::Rule::statement => {
                for statement2 in statements2_inner.into_inner() {
                    match statement2.as_rule() {
                        ast_generator::Rule::verb => {
                            i += 1;
                            verb_nr += 1;
                            //println!("{} -> <{}>", verb_nr, statement2.as_str());
                            match verb_nr {
                                1 => {
                                    assert_eq!(
                                        statement2.as_str(),
                                        "@foo"
                                    );
                                }
                                2 => {
                                    assert_eq!(
                                        statement2.as_str(),
                                        "@bar"
                                    );
                                }
                                //next statement
                                3 => {
                                    assert_eq!(
                                        statement2.as_str(),
                                        "@foo"
                                    );
                                }
                                4 => {
                                    assert_eq!(
                                        statement2.as_str(),
                                        "@tar"
                                    );
                                }
                                _ => {}
                            }
                        }
                        ast_generator::Rule::scope => {
                            i += 1;
                            let re = Regex::new(r"\{([ \n])*}")
                                .unwrap();
                            assert!(re
                                .is_match(statement2.as_str()));
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


#[test]
fn ident_can_start_with_underscore() {
    let input = r#"@__collect_inotify_mark"#;
    let result = ast_generator::parse_grammar(input);
    assert!(result.is_ok());

    assert_eq!(result.unwrap().as_str(), "@__collect_inotify_mark");
}