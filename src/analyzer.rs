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

    fn interpret_statements(&mut self, ast_nodes: Vec<AstNode>) -> (HashSet<FunctionNode>, u32) {
        let mut function_names: (HashSet<FunctionNode>, u32) = (HashSet::new(), 0);

        for ast in ast_nodes {
            let result =  self.interpret_statement(ast);
            if function_names.1 < result.1 {
                function_names.1 = result.1
            }
            for function in result.0.clone() {
                function_names.0.insert(function);
            }

        }
        function_names
    }

    fn interpret_statement(&mut self, ast: AstNode) -> (HashSet<FunctionNode>,u32) {
        let mut parents: HashSet<FunctionNode> = HashSet::new();
        let mut parent_filter: Vec<HashMap<FilterName, Regex>> = Vec::new();
        let mut child_names: HashSet<FunctionNode> = HashSet::new();

        let mut do_search = false;

        let mut search_grandparents = 0;

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
                                    let responde = self.interpret_statements(statements);
                                    child_names = responde.0;
                                    search_grandparents = responde.1;
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

        if parent_filter.clone().len() == 0 && child_names.clone().len() == 0 && do_search{
            return (HashSet::new(), search_grandparents + 1);

        }

        let mut parent_names:HashSet<FunctionNode> = HashSet::new();
        let mut has_parent_filter = true;
        if parent_filter.len() == 0 {
            has_parent_filter = false;
            let mut default_filter = HashMap::new();
            default_filter.insert(FilterName::Function, Regex::new(".").unwrap());
            parent_filter.push(default_filter);
        }
        parent_names = self.lang_server.find_func_name(parent_filter.clone());

        if do_search {
            if child_names.len() == 0 {
                let mut child_filter: Vec<HashMap<FilterName, Regex>> = Vec::new();
                child_filter.push(HashMap::new());

                child_names = self.lang_server.find_func_name(child_filter);
            }


            let mut child_names_with_parents: HashSet<String> = HashSet::new();
            for parent in parent_names.clone() {

                //self.graph.add_node(parent.clone().function_name.clone());
                let mut did_find_important_node = false;
                let mut matched_parents: HashSet<String> = HashSet::new();
                for child in child_names.to_owned() {
                    let connections = child.clone().match_strategy.do_match(parent.to_owned(), &mut self.lang_server);
                    for is_match in connections.clone() {
                        matched_parents.insert(is_match.0.clone());
                        self.graph.add_node(is_match.0.clone(), 1);
                        self.graph.add_node(is_match.1.clone(), 1);
                        did_find_important_node = self.graph.add_edge(is_match.0.clone(), is_match.1.clone());
                        child_names_with_parents.insert(is_match.1.clone());
                    }
                }
                parents.insert(FunctionNode{ function_name: matched_parents, match_strategy:parent.match_strategy });

                if did_find_important_node {
                    //remove unimportant nodes from graph
                    for child in child_names.clone() {
                        for node in self.graph.nodes.clone() {
                            if child.function_name.contains(&node.name.clone()) {
                                if node.times_used < 2 {
                                    self.graph.remove_node(node.clone());
                                }
                            }
                        }
                    }
                }
            }
            let node = ParentChildNode {
                function_name: child_names_with_parents.clone(),
            };
            let mut child_with_parents = FunctionNode { function_name: child_names_with_parents.clone(), match_strategy: Box::new(node) };

            while search_grandparents > 0 {
                let mut new_child_list = HashSet::new();

                let mut parent = child_with_parents.clone();

                let mut grand_children = HashSet::new();
                let mut child_filter: Vec<HashMap<FilterName, Regex>> = Vec::new();
                let mut default_filter = HashMap::new();
                default_filter.insert(FilterName::Function, Regex::new(".").unwrap());
                child_filter.push(default_filter);

                grand_children = self.lang_server.find_func_name(child_filter);


                for grand_child in grand_children {
                    let connections = parent.match_strategy.do_match(grand_child.clone(), &mut self.lang_server);
                    for connection in connections.clone() {
                        new_child_list.insert(connection.1.clone());

                        self.graph.add_node(connection.0.clone(), 1);
                        self.graph.add_node(connection.1.clone(), 1);
                        self.graph.add_edge(connection.0.clone(), connection.1.clone());
                    }
                }

                let node = ParentChildNode {
                    function_name: new_child_list.clone(),
                };
                child_with_parents = FunctionNode { function_name: new_child_list.clone(), match_strategy: Box::new(node) };

                search_grandparents -= 1;
            }
        }
        if has_parent_filter {
            for parent in parent_names.clone() {
                for name in parent.function_name.clone() {
                    self.graph.add_node(name, 1);
                }
            }
            parents = parent_names;
        } else {
            for parent in parents.clone() {
                for node in self.graph.nodes.clone() {
                    if parent.function_name.contains(&node.name.clone()) {
                        for edge in self.graph.pet_graph.edge_indices() {
                            if self.graph.pet_graph.edge_weight(edge).is_some() {
                                let endpoints = self.graph.pet_graph.edge_endpoints(edge).unwrap();
                                if endpoints.0 == endpoints.1 {
                                    if self.graph.pet_graph.node_weight(endpoints.0).unwrap().to_owned() == node.name {
                                        self.graph.pet_graph.remove_edge(edge);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        (parents, 0)
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