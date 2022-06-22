
use std::collections::{HashMap, HashSet};
use std::string::String;
use log::{log, Level, error};
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

#[derive(Eq, Hash, PartialEq, Debug)]
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
        let mut child_list: HashSet<String> = HashSet::new();
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
        self.lang_server.restart();
        if parent_filter.len() == 0 && do_search {
            if child_names.to_owned().len() == 0 {
                let node = ParentChildNode {
                    function_name: 1.to_string(),
                    document: "NONE".to_string()
                };
                return HashSet::from(
                    [FunctionNode { function_name: 1.to_string(), document: "NONE".to_string(), priority: 0, match_strategy: Box::new(node) }]
                );
            }
            else if child_names.to_owned().len() == 1 {
                let child = child_names.iter().last().unwrap();
                if child.document == "NONE" && child.function_name.as_str().parse::<i32>().is_ok(){
                    let i = child.function_name.as_str().parse::<i32>().unwrap() + 1;
                    let node = ParentChildNode {
                        function_name: i.to_string(),
                        document: "NONE".to_string()
                    };
                    return HashSet::from([FunctionNode { function_name: i.to_string(), document: "NONE".to_string(), priority: 0, match_strategy: Box::new(node) }]);
                }
            }
        }

        let mut search_grandparents = 0;
        if child_names.len() == 1 {
            let child=child_names.iter().last().unwrap().to_owned();
            if child.function_name.as_str().parse::<i32>().is_ok() && child.document == "NONE" {
                search_grandparents = child.function_name.as_str().parse::<i32>().unwrap();
                child_names.remove(&child);
            }
        }
        if  parent_filter.len() > 0 {
            let parent_names = self.lang_server.find_func_name(parent_filter);
            let mut i = 0;
            for parent in parent_names {
                self.graph.add_node(parent.clone().function_name.clone());
                if child_names.to_owned().len() > 0 {
                    let mut did_find_important_node = false;
                    for mut child in child_names.to_owned(){
                        println!("{:?}", child.function_name);
                        i += 1;
                        if i >= 10{
                            i = 0;
                            self.lang_server.restart();
                        }
                        if child.priority == 1 {
                            if parent.clone().match_strategy.do_match(child.to_owned(), &mut self.lang_server) {
                                parents.insert(parent.clone());
                                let a = self.graph.add_node(parent.function_name.clone());
                                let b = self.graph.add_node(child.function_name.clone());
                                self.graph.add_edge(a, b);
                                child.priority = 2;
                                child_names.remove(&child);
                                child_names.insert(child);
                                did_find_important_node = true;
                            }
                        }
                    }
                    if did_find_important_node {
                        //remove unimportant nodes from graph
                        for child in child_names.clone(){
                            if child.priority == 1 {
                                for node in self.graph.pet_graph.node_indices() {
                                    let node_name = self.graph.pet_graph.node_weight(node);
                                    if node_name.is_some(){
                                        if node_name.unwrap().to_owned() == child.function_name {
                                            self.graph.pet_graph.remove_node(node);
                                        }
                                    }
                                }
                            } else {
                                println!("x");
                            }
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
                            child_list.insert(child.clone().1);
                            let a = self.graph.add_node(parent.clone().function_name.clone());
                            let b = self.graph.add_node(child.1);
                            self.graph.add_edge(a, b);
                        }
                    } else {
                        parents.insert(parent.clone());
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
                    let prio: u32;
                    if parent.0.clone() == child.function_name {
                        prio = 2;
                    } else {
                        prio = 1;
                    }
                    parents.insert(FunctionNode{ function_name: parent.0.clone(), document: "".to_string(), priority: prio, match_strategy: Box::new(node) });

                    let a = self.graph.add_node(parent.0.clone());
                    let b = self.graph.add_node(child.function_name.clone());
                    self.graph.add_edge(a, b);
                }
            }
        }

        while search_grandparents > 0 {
            let mut new_child_list = HashSet::new();

            for child in child_list {
                let parent = child.clone();
                let grand_children = self.lang_server.search_connection_filter(
                    parent.clone(),
                    String::new(),
                );
                for grand_child in grand_children{
                    new_child_list.insert(grand_child.clone().1);
                    let a = self.graph.add_node(parent.clone());
                    let b = self.graph.add_node(grand_child.1);
                    self.graph.add_edge(a, b);
                }
            }
            self.lang_server.restart();

            child_list = new_child_list;
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