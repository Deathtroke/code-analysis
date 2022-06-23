use std::collections::{HashMap, HashSet};
use std::string::String;
use super::*;

use regex::Regex;
use crate::ast_generator::AstNode;
use crate::searcher::{ParentChildNode, FunctionNode};

pub struct Analyzer {
    pub graph : graph::Graph,
    lang_server : Box<dyn searcher::LSPServer>,
    //global_vars :HashSet<(String, HashSet<(String, String)>)>,
    //global_filter :HashSet<(String, String)>
}

#[derive(Eq, Hash, PartialEq, Debug, Clone)]
pub enum FilterName {
    Function,
    FunctionNameFromIdent,
    File,
    Forced,
}


impl Analyzer {
    pub fn new(lsp_server: Box<dyn searcher::LSPServer>) -> Analyzer {
        let p = Analyzer {
            graph: graph::Graph {
                pet_graph: petgraph::Graph::new(),
                nodes: HashSet::new(),
            },
            lang_server: lsp_server,
        };
        p
    }

    pub fn parse(&mut self, input: &str){
        let ast_result = ast_generator::parse_ast(input);
        if ast_result.is_ok() {
            for ast in ast_result.unwrap() {
                match ast {
                    AstNode::Statements(statements) => {
                        self.interpret_statements(statements);
                    }
                    _ => {}
                }
            }
        } else {
            panic!("unable to parse input: {:?}", ast_result.err());
        }
    }

    fn interpret_statements(&mut self, ast_nodes: Vec<AstNode>) -> HashSet<FunctionNode> {
        let mut function_names: HashSet<FunctionNode> = HashSet::new();

        for ast in ast_nodes {
            function_names = self.interpret_statement(ast);

        }
        function_names
    }

    fn interpret_statement(&mut self, ast: AstNode) -> HashSet<FunctionNode> {
        let mut parents: HashSet<FunctionNode> = HashSet::new();
        let mut parent_filter: Vec<HashMap<FilterName, Regex>> = Vec::new();
        let mut child_names: HashSet<FunctionNode> = HashSet::new();
        let mut do_search = false;

        match ast {
            AstNode::Statement { verb, scope } => {
                if verb.len() > 0 {
                    let filter = self.interpret_verb(verb);
                    parent_filter.push(filter.clone());
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

        if parent_filter.len() == 0 && do_search {
            if child_names.to_owned().len() == 0 {
                let node = ParentChildNode {
                    function_name: HashSet::from([1.to_string()]),
                    document: "NONE".to_string()
                };
                return HashSet::from(
                    [FunctionNode { function_name: HashSet::from([1.to_string()]), document: "NONE".to_string(), match_strategy: Box::new(node) }]
                );
            }
            else if child_names.to_owned().len() == 1 {
                let child = child_names.iter().last().unwrap().to_owned();
                if child.function_name.len() == 1 {
                    let count = child.function_name.iter().last().unwrap().to_owned();
                    if child.document == "NONE" && count.as_str().parse::<i32>().is_ok() {
                        let i = count.as_str().parse::<i32>().unwrap() + 1;
                        let node = ParentChildNode {
                            function_name: HashSet::from([i.to_string()]),
                            document: "NONE".to_string()
                        };
                        return HashSet::from([FunctionNode { function_name: HashSet::from([i.to_string()]), document: "NONE".to_string(), match_strategy: Box::new(node) }]);
                    }
                }
            }
        }


        let mut search_grandparents = 0;
        if child_names.len() == 1 {
            let child=child_names.clone().iter().last().unwrap().to_owned();
            for function_name in child.clone().function_name{
                if function_name.as_str().parse::<i32>().is_ok() && child.document == "NONE" {
                    search_grandparents = function_name.as_str().parse::<i32>().unwrap();
                    child_names.remove(&child);
                }
            }
        }

        let mut parent_names:HashSet<FunctionNode> = HashSet::new();
        if parent_filter.len() == 0 {
            let mut default_filter = HashMap::new();
            default_filter.insert(FilterName::Function, Regex::new(".").unwrap());
            parent_filter.push(default_filter);
        }
        parent_names = self.lang_server.find_func_name(parent_filter);

        if do_search {
            if child_names.len() == 0 {
                let mut child_filter: Vec<HashMap<FilterName, Regex>> = Vec::new();
                let mut default_filter = HashMap::new();
                default_filter.insert(FilterName::Function, Regex::new(".").unwrap());
                child_filter.push(default_filter);

                child_names = self.lang_server.find_func_name(child_filter);
            }
        }

        let mut i = 0;
        for parent in parent_names {
            //self.graph.add_node(parent.clone().function_name.clone());
            let mut did_find_important_node = false;
            for child in child_names.to_owned() {
                i += 1;
                if i >= 5 {
                    i = 0;
                    self.lang_server.restart();
                }
                let connections = parent.clone().match_strategy.do_match(child.to_owned(), &mut self.lang_server);
                for is_match in connections {
                    parents.insert(parent.clone());
                    self.graph.add_node(is_match.0.clone(), 1);
                    self.graph.add_node(is_match.1.clone(), 1);
                    self.graph.add_edge(is_match.0, is_match.1);
                    did_find_important_node = true;
                }
            }
            parents.insert(parent.clone());

            if did_find_important_node {
                //remove unimportant nodes from graph
                for child in child_names.clone() {
                    for node in self.graph.nodes.clone() {
                        if child.function_name.contains(&node.name.clone()) {
                            if node.priority == 2 {
                                for node_index in self.graph.pet_graph.node_indices() {
                                    if self.graph.pet_graph.node_weight(node_index).is_some() {
                                        if self.graph.pet_graph.node_weight(node_index).unwrap().to_owned() == node.name {
                                            self.graph.pet_graph.remove_node(node_index);
                                        }
                                        self.graph.nodes.remove(&node);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        while search_grandparents > 0 {
            let mut new_child_list = HashSet::new();

            for child in child_names {
                let mut parent = child.clone();

                let mut grand_children= HashSet::new();
                let mut child_filter: Vec<HashMap<FilterName, Regex>> = Vec::new();
                let mut default_filter = HashMap::new();
                default_filter.insert(FilterName::Function, Regex::new(".").unwrap());
                child_filter.push(default_filter);

                grand_children = self.lang_server.find_func_name(child_filter);


                for grand_child in grand_children {
                    let connections = parent.match_strategy.do_match(grand_child.clone(), &mut self.lang_server);
                    for connection in connections {
                        new_child_list.insert(grand_child.clone());

                        self.graph.add_node(connection.0.clone(), 1);
                        self.graph.add_node(connection.1.clone(), 1);
                        self.graph.add_edge(connection.0.clone(), connection.1.clone());
                    }
                }
            }
            self.lang_server.restart();

            child_names = new_child_list;
            search_grandparents -= 1;
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
mod analyzer_test;