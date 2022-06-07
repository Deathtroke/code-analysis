use pest::Parser;
use pest_derive::Parser;
use pest::iterators::Pairs;
use regex::Regex;

#[derive(Parser)]
#[grammar = "grammar.pest"]
struct MyParser;

#[derive(Debug,Clone)]
pub enum AstNode {
    Print(Box<AstNode>),
    Ident(String),
    Regex(Regex),
    NamedParameter {
        ident: Box<AstNode>,
        regex: Box<AstNode>,
    },
    Verb {
        ident: Box<AstNode>,
        named_parameter:Vec<AstNode>,
    },
    Scope(Box<AstNode>),
    Statement {
        verb: Vec<AstNode>,
        scope: Box<Option<AstNode>>,
    },
    Statements(Vec<AstNode>)

}

pub fn parse_grammar(input: &str) -> Result<Pairs<Rule>, pest::error::Error<Rule>> {
    let pair = MyParser::parse(Rule::query, input);
    //.expect("unsuccessful parse")
    //.next();
    pair
}

pub fn parse_ast(source: &str) -> Result<Vec<AstNode>, pest::error::Error<Rule>> {
    let mut ast: Vec<AstNode> = vec![];

    let pairs = parse_grammar( source)?;
    for pair in pairs {
        match pair.as_rule() {
            Rule::statements => {
                ast.push(AstNode::Print(Box::new(
                    AstNode::Statements(build_ast_from_statements(pair.into_inner()))
                )));
            }
            _ => {}
        }
    }

    Ok(ast)
}


fn build_ast_from_statements(pairs: pest::iterators::Pairs<Rule>) -> Vec<AstNode> {
    let mut statements : Vec<AstNode> = Vec::new();
    for pair in pairs{
        match pair.as_rule() {
            Rule::statement => statements.push(build_ast_from_statement(pair.into_inner())),
            _ => panic!("{:?}", pair),
        }
    }
    statements
}

fn build_ast_from_statement(pairs: pest::iterators::Pairs<Rule>) -> AstNode {
    let mut verb = vec![];
    let mut scope = Option::None;

    for pair in pairs {

        match pair.as_rule() {
            Rule::verb => verb.push(build_ast_from_verb(pair.into_inner())),
            Rule::scope => {scope = Some(build_ast_from_scope(pair.into_inner().next().unwrap()))},
            _=>{}
        }
    }

    AstNode::Statement {
        verb,
        scope:Box::new(scope)
    }

}

fn build_ast_from_verb(pairs: pest::iterators::Pairs<Rule>) -> AstNode {
    let mut named_parameter:Vec<AstNode> = vec![];
    let mut ident_str = String::new();

    for pair in pairs {
        match pair.as_rule() {
            Rule::ident => {
                ident_str = pair.as_str().to_string();
            },
            Rule::named_parameter => {
                let parameter = build_ast_from_named_parameter(pair.into_inner());
                named_parameter.push(
                    AstNode::NamedParameter {
                        ident: Box::new(parameter.0),
                        regex: Box::new(parameter.1)
                    }
                );
            },
            _ => { },
        }
    }
    AstNode::Verb {
        ident: Box::new(AstNode::Ident(ident_str)),
        named_parameter,
    }
}

fn build_ast_from_named_parameter(pairs: pest::iterators::Pairs<Rule>) -> (AstNode, AstNode) {
    let mut ident_str = String::new();
    let mut regex_expr = Regex::new(".").unwrap();
    for pair in pairs {
        match pair.as_rule() {
            Rule::ident => {
                ident_str = pair.as_str().clone().to_string();
            },
            Rule::regex => {
                regex_expr = Regex::new(pair.as_str()).unwrap();
            },
            _ => {},
        }
    }
    (
        AstNode::Ident(ident_str),
        AstNode::Regex(regex_expr)
    )

}

fn build_ast_from_scope(pair: pest::iterators::Pair<Rule>) -> AstNode {
    match pair.as_rule() {
        Rule::statements => {
            AstNode::Scope(Box::new(
                AstNode::Statements(build_ast_from_statements(pair.into_inner()))
            ))
        },
        _ => panic!("{:?}", pair),
    }
}

#[cfg(test)]
mod ast_test;