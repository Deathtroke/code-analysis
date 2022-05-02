// Note this useful idiom: importing names from outer (for mod tests) scope.
use super::*;
use pest::iterators::Pair;


#[test]
fn test_grammar_simple1() {
    let input = r#"parent of "func1""#;
    let pair: Pair<parser::Rule> = parser::parse(input);
    //println!("{:?}", pair);
    for inner_pair in pair.into_inner() {
        match inner_pair.as_rule() {
            parser::Rule::command_type => {
                assert_eq!(inner_pair.as_str(), "parent");
            }
            parser::Rule::function_name => {
                assert_eq!(inner_pair.as_str(), r#""func1""#);
            }
            _ => {}
        }
    }
    //assert_eq!(pair, pair);
}

#[test]
fn test_grammar_simple2() {
    let input = r#"child of "func2""#;
    let pair: Pair<parser::Rule> = parser::parse(input);
    //println!("{:?}", pair);
    for inner_pair in pair.into_inner() {
        match inner_pair.as_rule() {
            parser::Rule::command_type => {
                assert_eq!(inner_pair.as_str(), "child");
            }
            parser::Rule::function_name => {
                assert_eq!(inner_pair.as_str(), r#""func2""#);
            }
            _ => {}
        }
    }
    //assert_eq!(pair, pair);
}

#[test]
fn test_grammar1() {
    let input = r#"parent of "func1" where file="123""#;
    let pair: Pair<parser::Rule> = parser::parse(input);
    //println!("{:?}", pair);
    for inner_pair in pair.into_inner() {
        match inner_pair.as_rule() {
            parser::Rule::command_type => {
                assert_eq!(inner_pair.as_str(), "parent");
            }
            parser::Rule::function_name => {
                assert_eq!(inner_pair.as_str(), r#""func1""#);
            }
            parser::Rule::filter => {
                assert_eq!(inner_pair.as_str(), r#"file="123""#);
            }
            _ => {}
        }
    }
    //assert_eq!(pair, pair);
}

#[test]
fn test_grammar2() {
    let input = r#"parent of "func2" as "function_x""#;
    let pair: Pair<parser::Rule> = parser::parse(input);
    //println!("{:?}", pair);
    for inner_pair in pair.into_inner() {
        match inner_pair.as_rule() {
            parser::Rule::command_type => {
                assert_eq!(inner_pair.as_str(), "parent");
            }
            parser::Rule::function_name => {
                assert_eq!(inner_pair.as_str(), r#""func2""#);
            }
            parser::Rule::overwrite_name => {
                assert_eq!(inner_pair.as_str(), r#""function_x""#);
            }
            _ => {}
        }
    }
    //assert_eq!(pair, pair);
}

#[test]
fn test_grammar_complex1() {
    let input = r#"parent of {parent of "func1" where @new:filter(file = "abc")} where @filter"#;
    let pair: Pair<parser::Rule> = parser::parse(input);
    //println!("{:?}", pair);
    for inner_pair in pair.into_inner() {
        match inner_pair.as_rule() {
            parser::Rule::command_type => {
                assert_eq!(inner_pair.as_str(), "parent");
            }

            parser::Rule::macro_filter => {
                assert_eq!(inner_pair.as_str(), "@filter");
            }

            parser::Rule::function_name => {
                assert_eq!(inner_pair.as_str(), r#""func1""#);
            }
            _ => {}
        }
    }
    //assert_eq!(pair, pair);
}