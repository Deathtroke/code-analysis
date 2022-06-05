use pest::Parser;
use pest_derive::Parser;
use pest::iterators::{Pair, Pairs};
use std::collections::{HashMap, HashSet};
use std::string::String;
use log::{log, Level};
use super::*;

use regex::Regex;

use crate::searcher::{ParentChildNode, FunctionNode, ForcedNode};

#[derive(Parser)]
#[grammar = "grammar.pest"]
struct MyParser;

pub struct PestParser {
    pub graph : graph::Graph,
    lang_server : Box<dyn searcher::LSPServer>,
    //global_vars :HashSet<(String, HashSet<(String, String)>)>,
    //global_filter :HashSet<(String, String)>
}

#[derive(Eq, Hash, PartialEq, Debug)]
pub enum FilterName {
    Function,
    File,
    Forced,
}

pub fn parse_grammar(input: &str) -> Result<Pairs<Rule>, pest::error::Error<Rule>> {
    let pair = MyParser::parse(Rule::query, input);
        //.expect("unsuccessful parse")
        //.next();
    pair
}

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



impl PestParser {
    pub fn new(lsp_server: Box<dyn searcher::LSPServer>) -> PestParser {
        let p = PestParser {
            graph: graph::Graph {
                edges: HashSet::new(),
            },
            lang_server: lsp_server,
            //global_vars:HashSet::new(),
            //global_filter:HashSet::new()
        };
        p
    }

    pub fn parse(&mut self, input: &str) -> HashSet<FunctionNode>{
        let ast_result = parse_ast(input);
        if ast_result.is_ok() {
            self.interpret_statements(ast_result.unwrap())
        } else {
            log!(Level::Error, "unable to parse input: {:?}", ast_result.err());
            HashSet::new()
        }
    }

    fn interpret_statements(&mut self, ast_nodes: Vec<AstNode>) -> HashSet<FunctionNode> {
        let mut function_names: HashSet<FunctionNode> = HashSet::new();
        //let mut overwrite_name : String = "".to_string();


        for ast in ast_nodes {
            match ast {
                AstNode::Print(print) => {
                    match *print {
                        AstNode::Statements(statements) => {
                            function_names = self.interpret_statements(statements);

                        },
                        _ => {}
                    }
                }
                _ => {
                    function_names = self.interpret_statement(ast, function_names);
                }
            }
        }
        function_names
    }

    /*
    fn parse_global_definition(&mut self, pair: Pair<Rule>){
        let mut param_type = "";
        for inner_pair in pair.to_owned().into_inner() {
            match inner_pair.as_rule() {
                Rule::predefined_identifier => {
                    param_type = inner_pair.as_str();
                }
                Rule::define_filter_variable => {
                    let mut var_name = "";
                    let mut var_filter: HashSet<(String, String)> = HashSet::new();
                    if param_type == "define" {
                        for define_filter_variable in inner_pair.into_inner() {
                            match define_filter_variable.as_rule() {
                                Rule::identifier => {
                                    var_name = define_filter_variable.as_str();
                                }
                                Rule::named_parameter => {
                                    var_filter.insert(self.parse_named_parameter(define_filter_variable));
                                }
                                _ => {}

                            }
                        }
                    }else{
                        println!("unexpected define_filter_variable") ;
                    }
                }
                Rule::argument => {
                    self.global_filter.insert((param_type.to_string(), inner_pair.to_string()));
                }
                _ => {}
            }
        }
    }
*/

    fn interpret_statement(&mut self, ast: AstNode, mut parents: HashSet<FunctionNode>) -> HashSet<FunctionNode> {
        let mut parent_filter: Vec<HashMap<FilterName, Regex>> = Vec::new();
        let mut child_names: HashSet<FunctionNode> = HashSet::new();
        let mut do_search = false;
        match ast {
            AstNode::Statement { verb, scope } => {
                if verb.len() > 0 {
                    let filter = self.interpret_verb(verb);
                    parent_filter.push(filter);
                }
                if scope.is_some() {
                    do_search = true;
                    match scope.unwrap() {
                        AstNode::Scope(scope_inner) => {
                            match *scope_inner {
                                AstNode::Statements(statements) => {
                                    child_names = self.interpret_statements(statements);
                                }
                                _=>{}
                            }
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }

        if  parent_filter.len() > 0 {
            let parent_names = self.lang_server.find_func_name(parent_filter);
            for parent in parent_names {
                if child_names.to_owned().len() > 0 {
                    for child in child_names.to_owned(){
                        if parent.clone().match_strategy.do_match(child.to_owned(), &mut self.lang_server) {
                            parents.insert(parent.clone());
                            self.graph.insert_edge(None, parent.function_name.clone(), child.function_name.clone());
                        }
                    }
                } else {
                    if do_search {
                        let children = self.lang_server.search_child_single_document_filter(
                            Regex::new(parent.function_name.clone().as_str()).unwrap(),
                            HashMap::new(),
                            parent.document.clone().as_str()
                        );
                        if children.len() > 0 {
                            parents.insert(parent.clone());
                            for child in children{
                                self.graph.insert_edge(None, parent.clone().function_name.clone(), child.1);
                            }
                        }
                    } else {
                        parents.insert(parent);
                    }
                }
            }
        } else {
            for child in child_names {
                for parent in self.lang_server.search_parent(child.function_name.clone()) {
                    let node = ParentChildNode {
                        function_name: parent.clone(),
                        document: "".to_string()
                    };
                    parents.insert(FunctionNode{ function_name: parent.clone(), document: "".to_string(), match_strategy: Box::new(node) });

                    self.graph.insert_edge(None, parent.clone(), child.function_name.clone());
                }
            }
        }
        parents
    }

    fn interpret_verb(&mut self, ast_nodes: Vec<AstNode>) -> HashMap<FilterName, Regex>{
        let mut filter: HashMap<FilterName, Regex> = HashMap::new();
        for ast in ast_nodes {
            match ast {
                AstNode::Verb { ident,named_parameter } =>{
                    match *ident {
                        AstNode::Ident(ident) => {
                            match ident.as_str() {
                                "filter" => {
                                    for parameter in named_parameter {
                                        let filter_option = self.interpret_define_options(parameter);
                                        if filter_option.is_some() {
                                            let filter_option_unwrap = filter_option.unwrap();
                                            filter.insert(filter_option_unwrap.0, filter_option_unwrap.1);
                                        }
                                    }
                                }
                                "forced" => {
                                    filter.insert(FilterName::Forced, Regex::new("TRUE").unwrap());
                                }
                                _ => {
                                    filter.insert(FilterName::Function, Regex::new(ident.as_str()).unwrap());
                                }
                            }
                        }
                        _ => {}
                    }

                }
                _ => {}
            }
        }
        filter
    }

    fn interpret_define_options(&mut self, ast: AstNode) -> Option<(FilterName, Regex)> {
        let mut filter_name = String::new();
        let mut value = Regex::new(".").unwrap();
        match ast {
            AstNode::NamedParameter { ident, regex } => {
                match *ident {
                    AstNode::Ident(ident) => {
                        filter_name = ident.to_owned();
                    }
                    _ =>{}
                }

                match *regex {
                    AstNode::Regex(regex) => {
                        value = regex;
                    }
                    _ =>{}
                }
            }
            _ => {}
        }

        match filter_name.to_lowercase().as_str(){
            "function" => {
                Some(
                    (FilterName::Function,
                    value)
                )
            }
            "file" => {
                Some(
                    (FilterName::File,
                     value)
                )
            }
            _ => {
                None
            }
        }
    }
}

#[cfg(test)]
mod parser_test;
#[cfg(test)]
mod ast_test;