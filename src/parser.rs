use pest::Parser;
use pest_derive::Parser;
use pest::iterators::{Pair, Pairs};
use std::collections::{HashMap, HashSet};
use std::string::String;
use super::*;

use regex::Regex;

use crate::searcher::{ParentChildNode, FunctionNode};

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
}

pub fn parse_grammar(input: &str) -> Result<Pairs<Rule>, pest::error::Error<Rule>> {
    let pair = MyParser::parse(Rule::query, input);
        //.expect("unsuccessful parse")
        //.next();
    pair
}

#[derive(Debug)]
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
                ast.push(AstNode::Print(Box::new(build_ast_from_statements(pair.into_inner().next().unwrap()))));
            }
            _ => {}
        }
    }

    Ok(ast)
}


fn build_ast_from_statements(pair: pest::iterators::Pair<Rule>) -> AstNode {
    match pair.as_rule() {
        Rule::statement => build_ast_from_statement(pair.into_inner()),
        _ => panic!("{:?}", pair),
    }
}

fn build_ast_from_statement(pairs: pest::iterators::Pairs<Rule>) -> AstNode {
    let mut verb = vec![];
    let mut scope = Option::None;

    for pair in pairs {
        match pair.as_rule() {
            Rule::verb => verb.push(build_ast_from_verb(pair.into_inner().next().unwrap())),
            Rule::scope => {scope = Some(build_ast_from_scope(pair.into_inner().next().unwrap()))},
            _=>{}
        }
    }

    AstNode::Statement {
        verb,
        scope:Box::new(scope)
    }

}

fn build_ast_from_verb(pair: pest::iterators::Pair<Rule>) -> AstNode {
    let mut named_parameter:Vec<AstNode> = vec![];
    let mut ident_str = String::new();

    match pair.as_rule() {
        Rule::ident => {
            ident_str = pair.as_str().to_string();
        },
        Rule::named_parameter => {named_parameter = build_ast_from_named_parameter(pair.into_inner())},
        _ => { },
    }

    AstNode::Verb {
        ident: Box::new(AstNode::Ident(ident_str)),
        named_parameter,
    }
}

fn build_ast_from_named_parameter(pairs: pest::iterators::Pairs<Rule>) -> Vec<AstNode> {
    let mut named_parameters:Vec<AstNode> = vec![];
    for pair in pairs {
        let mut ident_str = String::new();
        let mut regex_expr = Regex::new(".").unwrap();
        match pair.as_rule() {
            Rule::ident => {
                let mut inner_pair = pair.into_inner();
                ident_str = inner_pair.next().unwrap().as_str().to_string();
            },
            Rule::regex => {
                let mut inner_pair = pair.into_inner();
                regex_expr = Regex::new(inner_pair.next().unwrap().as_str()).unwrap();
            },
            _ => {},
        }
        named_parameters.push(AstNode::NamedParameter {
            ident: Box::new(AstNode::Ident(ident_str)),
            regex: Box::new(AstNode::Regex(regex_expr))
        });
    }
    named_parameters

}

fn build_ast_from_scope(pair: pest::iterators::Pair<Rule>) -> AstNode {
    match pair.as_rule() {
        Rule::statements => build_ast_from_statements(pair.into_inner().next().unwrap()),
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
        let pair = parse_grammar(input);
        if pair.is_ok() {
            self.interpret_statements(pair.unwrap().next().unwrap())
        } else {
            println!("unable to parse input: {:?}", pair.err());
            HashSet::new()
        }
    }

    fn interpret_statements(&mut self, pair: Pair<Rule>) -> HashSet<FunctionNode> {
        let mut function_names: HashSet<FunctionNode> = HashSet::new();
        //let mut overwrite_name : String = "".to_string();

        for inner_pair in pair.to_owned().into_inner() {
            match inner_pair.as_rule() {
                Rule::statement =>{
                    function_names = self.interpret_statement(inner_pair, function_names);
                }
                _ => {}
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

    fn interpret_statement(&mut self, pair: Pair<Rule>, mut parents: HashSet<FunctionNode>) -> HashSet<FunctionNode> {
        let mut parent_filter: Vec<HashMap<FilterName, String>> = Vec::new();
        let mut child_names: HashSet<FunctionNode> = HashSet::new();
        let mut do_search = false;
        for inner_pair in pair.to_owned().into_inner() {
            match inner_pair.as_rule() {
                Rule::verb => {
                    let filter = self.interpret_verb(inner_pair);
                    parent_filter.push(filter);
                }
                Rule::scope => {
                    do_search = true;
                    let scope = inner_pair.into_inner().nth(0).unwrap(); //there is always a nth(0) -> the found scope pair
                    match scope.as_rule() {
                        Rule::statements => {
                            child_names = self.interpret_statements(scope);
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
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
                    let prent_child_edge = ParentChildNode {
                        function_name: parent.clone(),
                        document: "".to_string()
                    };
                    parents.insert(FunctionNode{ function_name: parent.clone(), document: "".to_string(), match_strategy: Box::new(prent_child_edge) });
                    self.graph.insert_edge(None, parent.clone(), child.function_name.clone());
                }
            }
        }
        parents
    }

    fn interpret_verb(&mut self, pair: Pair<Rule>) -> HashMap<FilterName, String>{
        let mut filter: HashMap<FilterName, String> = HashMap::new();
        for inner_pair in pair.to_owned().into_inner() {
            match inner_pair.as_rule() {
                Rule::ident => {
                    let ident = inner_pair.as_str();
                    filter.insert(FilterName::Function, ident.to_string());
                }
                Rule::named_parameter => {
                    let filter_option = self.interpret_define_options(inner_pair);
                    if filter_option.is_some(){
                        let filter_option_unwrap = filter_option.unwrap();
                        filter.insert(filter_option_unwrap.0, filter_option_unwrap.1);
                    }
                }
                _ => {}
            }
        }

        filter
    }

    fn interpret_define_options(&mut self, pair: Pair<Rule>) -> Option<(FilterName, String)> {
        let mut filter_name = "";
        let mut value = "";
        for inner_pair in pair.to_owned().into_inner() {
            match inner_pair.as_rule() {
                Rule::ident => {
                    filter_name = inner_pair.as_str();
                }
                Rule::regex => {
                    value = inner_pair.as_str();
                }
                _ => {}
            }
        }

        match filter_name.to_lowercase().as_str(){
            "function" => {
                Some(
                    (FilterName::Function,
                    value.to_string())
                )
            }
            "file" => {
                Some(
                    (FilterName::File,
                     value.to_string())
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