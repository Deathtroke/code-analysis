
use std::collections::{HashMap, HashSet};
use std::string::String;
use log::{log, Level};
use super::*;

use regex::Regex;
use crate::ast_generator::AstNode;
use crate::searcher::{ParentChildNode, FunctionNode};

pub struct PestParser {
    pub graph : graph::Graph,
    lang_server : Box<dyn searcher::LSPServer>,
    //global_vars :HashSet<(String, HashSet<(String, String)>)>,
    //global_filter :HashSet<(String, String)>
}

#[derive(Eq, Hash, PartialEq, Debug)]
pub enum FilterName {
    Function,
    FunctionNameFromIdent,
    File,
    Forced,
}


impl PestParser {
    pub fn new(lsp_server: Box<dyn searcher::LSPServer>) -> PestParser {
        let p = PestParser {
            graph: graph::Graph {
                pet_graph: petgraph::Graph::new(),
            },
            lang_server: lsp_server,
        };
        p
    }

    pub fn parse(&mut self, input: &str) -> HashSet<FunctionNode>{
        let ast_result = ast_generator::parse_ast(input);
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
                            let a = self.graph.add_node(parent.function_name.clone());
                            let b = self.graph.add_node(child.function_name.clone());
                            self.graph.pet_graph.add_edge(a, b, ());
                        }
                    }
                } else {
                    if do_search {
                        let children = self.lang_server.search_connection_filter(
                            parent.function_name.clone(),
                            String::new(),
                        );
                        parents.insert(parent.clone());
                        for child in children{
                            let a = self.graph.add_node(parent.clone().function_name.clone());
                            let b = self.graph.add_node(child.1);
                            self.graph.pet_graph.add_edge(a, b, ());
                        }
                    } else {
                        parents.insert(parent.clone());
                        self.graph.add_node(parent.clone().function_name.clone());
                    }
                }
            }
        } else {
            for child in child_names {
                let found_parents = self.lang_server.search_connection_filter(String::new(),child.function_name.clone());
                for parent in found_parents {
                    let node = ParentChildNode {
                        function_name: parent.0.clone(),
                        document: "".to_string()
                    };
                    parents.insert(FunctionNode{ function_name: parent.0.clone(), document: "".to_string(), match_strategy: Box::new(node) });

                    let a = self.graph.add_node(parent.0.clone());
                    let b = self.graph.add_node(child.function_name.clone());
                    self.graph.pet_graph.add_edge(a, b, ());
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
                                    filter.insert(FilterName::FunctionNameFromIdent, Regex::new(ident.as_str()).unwrap());
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

    pub fn close_lsp(&mut self) {
        self.lang_server.close();
    }
}

#[cfg(test)]
mod parser_test;