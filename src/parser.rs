use std::borrow::Borrow;
use pest::Parser;
use pest_derive::Parser;
use pest::iterators::{Pair, Pairs};
use std::collections::{HashMap, HashSet};
use std::string::String;
use super::*;
use std::fs;
use std::fs::File;
use std::io::prelude::*;


use regex::Regex;

use crate::searcher::{DefaultEdge, ForcedEdge, FunctionEdge, LSPServer, MatchFunctionEdge};

#[derive(Parser)]
#[grammar = "grammar.pest"]
struct MyParser;

pub struct parser {
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

impl parser {
    pub fn new(project_path: String, lsp_server: Box<dyn searcher::LSPServer>) -> parser {
        let p = parser{
            graph: graph::Graph {
                edges: HashSet::new(),
            },
            lang_server: lsp_server,
            //global_vars:HashSet::new(),
            //global_filter:HashSet::new()
        };
        p
    }

    pub fn parse(&mut self, input: &str) -> HashSet<FunctionEdge>{
        let pair = parse_grammar(input);
        if pair.is_ok() {
            self.parse_statements(pair.unwrap().next().unwrap())
        } else {
            println!("unable to parse input: {:?}", pair.err());
            HashSet::new()
        }
    }

    fn parse_statements(&mut self, pair: Pair<Rule>) -> HashSet<FunctionEdge> {
        let mut function_names: HashSet<FunctionEdge> = HashSet::new();
        //let mut overwrite_name : String = "".to_string();

        for inner_pair in pair.to_owned().into_inner() {
            match inner_pair.as_rule() {
                Rule::statement =>{
                    function_names = self.parse_statement(inner_pair,function_names);
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

    fn parse_statement (&mut self, pair: Pair<Rule>, mut parents: HashSet<FunctionEdge>) -> HashSet<FunctionEdge> {
        let mut parent_filter: Vec<HashMap<FilterName, String>> = Vec::new();
        let mut child_names: HashSet<FunctionEdge> = HashSet::new();
        let mut do_search = false;
        for inner_pair in pair.to_owned().into_inner() {
            match inner_pair.as_rule() {
                Rule::verb => {
                    let filter = self.parse_verb(inner_pair);
                    parent_filter.push(filter);
                }
                Rule::scope => {
                    do_search = true;
                    let scope = inner_pair.into_inner().nth(0).unwrap(); //there is always a nth(0) -> the found scope pair
                    match scope.as_rule() {
                        Rule::statements => {
                            child_names = self.parse_statements(scope);
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
        }
        //println!("parents {:?}", parent_filter);
        //println!("children {:?}", child_names);

        if  parent_filter.len() > 0 {
            let parent_names = self.lang_server.find_func_name(parent_filter);
            for mut parent in parent_names {
                if child_names.clone().len() > 0 {
                    for mut child in child_names.to_owned(){
                        if parent.clone().match_strategy.do_match(child.to_owned()) {
                            parents.insert(parent.clone());
                            self.graph.insert_edge(None, parent.function_name.clone(), child.function_name.clone());
                        }
                    }
                } else {
                    if do_search {
                        let children = self.lang_server.search_child(parent.function_name.clone());
                        if children.len() > 0 {
                            parents.insert(parent.clone());
                            for child in children{
                                self.graph.insert_edge(None, parent.function_name.clone(), child);
                            }
                        }
                    } else {
                        parents.insert(parent);
                    }
                }
            }
        } else {
            for mut child in child_names {
                for parent in self.lang_server.search_parent(child.function_name.clone()) {
                    parents.insert(FunctionEdge{ function_name: parent.clone(), document: "".to_string(), match_strategy: Box::new(DefaultEdge{lsp_server: &self.lang_server}) });
                    self.graph.insert_edge(None, parent.clone(), child.function_name.clone());
                }
            }
        }
        parents
    }

    fn parse_verb(&mut self, pair: Pair<Rule>) -> HashMap<FilterName, String>{
        let mut filter: HashMap<FilterName, String> = HashMap::new();
        let mut ident = "";
        for inner_pair in pair.to_owned().into_inner() {
            match inner_pair.as_rule() {
                Rule::ident => {
                    ident = inner_pair.as_str();
                    filter.insert(FilterName::Function, ident.to_string());
                }
                Rule::named_parameter => {
                    let filter_option = self.parse_define_options(inner_pair);
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

    fn parse_define_options(&mut self, pair: Pair<Rule>) -> Option<(FilterName, String)> {
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