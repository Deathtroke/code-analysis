use pest::Parser;
use pest_derive::Parser;
use pest::iterators::Pair;
use std::borrow::Borrow;
use std::collections::HashSet;
use std::string::String;
use graphviz_rust::{exec, parse, print};
use dot_structures::*;
use dot_generator::*;
use graphviz_rust::attributes::packmode::graph;
use super::*;
use graphviz_rust::printer::PrinterContext;
use graphviz_rust::cmd::{CommandArg, Format};
use crate::lang_server::LanguageServer;

#[derive(Parser)]
#[grammar = "grammar.pest"]
struct MyParser;

pub struct parser {
    pub graph :HashSet<(String, String)>,
    //lang_server : Box<dyn LanguageServer>,
    //global_vars :HashSet<(String, HashSet<(String, String)>)>,
    //global_filter :HashSet<(String, String)>
}

pub fn parse_grammar(input: &str) -> Pair<Rule> {
    let pair = MyParser::parse(Rule::query, input)
        .expect("unsuccessful parse")
        .next().unwrap();
    pair
}

impl parser {
    pub fn new() -> parser {
        let mut p = parser{
            graph:HashSet::new(),
            //lang_server: lang_server::LanguageServerLauncher::new()
            //    .server("/usr/bin/clangd".to_owned())
            //    .project("/Users/hannes.boerner/Downloads/criu-criu-dev".to_owned())
            //    //.languages(language_list)
            //    .launch()
            //    .expect("Failed to spawn clangd")
            //global_vars:HashSet::new(),
            //global_filter:HashSet::new()
        };
        //p.lang_server.initialize();
        p
    }

    pub fn parse(&mut self, input: &str) -> HashSet<String>{
        let pair = parse_grammar(input);
        self.parse_statements(pair)
    }

    fn parse_statements(&mut self, pair: Pair<Rule>) -> HashSet<String> {
        let mut function_names: HashSet<String> = HashSet::new();
        //let mut overwrite_name : String = "".to_string();

        for inner_pair in pair.to_owned().into_inner() {
            match inner_pair.as_rule() {
                Rule::statement =>{
                    function_names = self.parse_statement(inner_pair,function_names.clone());
                }
                /*
                Rule::function_name => {
                    for function_nam_pair in inner_pair.into_inner() {
                        match function_nam_pair.as_rule() {
                            Rule::string => {
                                function_names.insert(function_nam_pair.as_str().to_string());
                            }
                            _ => {}
                        }
                    }
                }
                Rule::functions => {
                    function_names = self.parse_function(inner_pair);
                }
                Rule::where_filter => {

                }
                Rule::overwrite => {
                    for overwirde_pair in inner_pair.into_inner() {
                        match overwirde_pair.as_rule() {
                            Rule::overwrite_name => {
                                overwrite_name = overwirde_pair.to_string();
                            }
                            _ => {}
                        }
                    }
                }
                */
                _ => {}
            }
        }
        let mut output: HashSet<String> = HashSet::new();
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

    fn parse_named_parameter(&mut self, pair: Pair<Rule>) -> (String, String) {
        let mut attribute :String = String::new();
        let mut value:String = String::new();
        for inner_pair in pair.to_owned().into_inner() {
            match inner_pair.as_rule() {
                Rule::attribute => {
                    attribute = inner_pair.to_string();
                }
                Rule::value => {
                    value = inner_pair.to_string();
                }
                _ => {}
            }
        }
        (attribute, value)
    }
    */

    fn parse_statement (&mut self, pair: Pair<Rule>, mut parents: HashSet<String>) -> HashSet<String> {
        let mut parent_names: HashSet<String> = HashSet::new();
        let mut child_names: HashSet<String> = HashSet::new();
        let mut do_search = false;
        for inner_pair in pair.to_owned().into_inner() {
            match inner_pair.as_rule() {
                Rule::verb => {
                    let mut name = inner_pair.as_str();
                    name = name.strip_prefix("@").unwrap();
                    while name.chars().last().unwrap() == ' '{
                        name = name.strip_suffix(" ").unwrap();
                    }
                    parent_names.insert(name.to_string());
                }
                Rule::scope => {
                    do_search = true;
                    let scope = inner_pair.into_inner().nth(0).unwrap();
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
        if  parent_names.len() > 0 {
            for parent in parent_names {
                if child_names.len() > 0 {
                    for child in child_names.clone() {
                        if self.paren_child_exists(parent.clone(), child) {
                            parents.insert(parent.clone());
                        }
                    }
                } else {
                    if do_search {
                        let children = self.search_child(parent.clone());
                        if children.len() > 0 {
                            parents.insert(parent.clone());
                        }
                    } else {
                        parents.insert(parent.clone());
                    }
                }
            }
        } else {
            for child in child_names {
                for parent in self.search_parent(child) {
                    parents.insert(parent.clone());
                }
            }
        }
        parents
    }
/*
    fn parse_requests(&mut self, pair: Pair<Rule>) {
        let mut child_filter: HashSet<(String, String)> = HashSet::new();
        let mut parent_filter: HashSet<(String, String)> = HashSet::new();
        for inner_pair in pair.to_owned().into_inner() {
            match inner_pair.as_rule() {
                Rule::request_expr => {
                    let mut found_function_filter = false;
                    let mut found_child_expr = false;
                    let mut specified_child_expr = false;
                    for inner_pair in pair.to_owned().into_inner() {
                        match inner_pair.as_rule() {
                            Rule::function_filter => {
                                //Parent
                                found_function_filter = true;
                                for function_filter in inner_pair.into_inner() {
                                    match function_filter.as_rule() {
                                        Rule::function_name => {
                                            parent_filter.insert(("name".to_string(), function_filter.to_string()));
                                        }
                                        Rule::filter_option => {
                                            self.parse_filter_option(function_filter);
                                        }
                                        _ => {}
                                    }
                                }
                            }
                            Rule::function_filte => {
                                //Child
                                found_child_expr = true;
                                for child_expr in inner_pair.into_inner() {
                                    match child_expr.as_rule() {
                                        Rule::requests => {
                                            specified_child_expr = true;
                                            self.parse_requests(child_expr);
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
    }

    fn parse_filter_option(&mut self, pair: Pair<Rule>, mut filter: HashSet<(String, String)>){
        let mut predefined_identifier_text = "";
        for inner_pair in pair.to_owned().into_inner() {
            match inner_pair.as_rule() {
                Rule::predefined_identifier => {
                    predefined_identifier_text = inner_pair.as_str();
                }
                Rule::named_parameter => {
                    if predefined_identifier_text =="filter" {
                        filter.insert(self.parse_named_parameter(inner_pair));
                    }
                }
                Rule::identifier => {
                    //self.global_vars.iter().any(|(v,x)| v == inner_pair.as_str())
                    for var in self.global_vars {
                        if var.0 == inner_pair.to_string(){
                            filter = var.1;
                            break;
                        }
                    }
                }
                _ => {}
            }
        }
    }
*/
    fn search_parent(&mut self, search_target: String)  -> HashSet<String>{
        #[cfg(test)]
            let parents :HashSet<String> = HashSet::from(["parent1".to_string(), "parent2".to_string()]);
        #[cfg(not(test))]
            let parents :HashSet<String> = self.search_parents(search_target.clone());
        for parent in parents.clone() {
            self.graph.insert((parent, search_target.clone()));

        }
        parents
    }

    fn search_child(&mut self, search_target: String)  -> HashSet<String>{
        #[cfg(test)]
            let children :HashSet<String> = HashSet::from(["child1".to_string(), "child2".to_string()]);
        #[cfg(not(test))]
            let children :HashSet<String> = self.search_children(search_target.clone());
        for child in children.clone() {
            self.graph.insert((search_target.clone(), child));
        }
        children
    }

    fn paren_child_exists(&mut self, parent: String, child: String) -> bool{
        #[cfg(test)]
            let result = (parent == "parent1" || parent == "parent2") &&
            (child == "child1" || child == "child2") ;
        #[cfg(not(test))]
            let result = self.search_children(parent.clone()).contains(child.as_str());

        if result {
            self.graph.insert((parent.clone(), child.clone()));
        }

        result
    }

    /*fn parse_function(&mut self, pair: Pair<Rule>)  -> HashSet<String>{
        let mut function_names: HashSet<String> = HashSet::new();
        for functions_pair in pair.into_inner() {
            match functions_pair.as_rule() {
                Rule::extra_command => {
                    //filter out {, } and whitespaces so only the next command can
                    for extra_command_pair in functions_pair.into_inner() {
                        match extra_command_pair.as_rule() {
                            Rule::command => {
                                function_names = self.parse_command(extra_command_pair);
                            }
                            _ => {}
                        }
                    }
                }
                Rule::function_name => {
                    function_names.insert(functions_pair.to_string());
                }
                _ => {}
            }
        }
        function_names
    }*/

    pub fn graph_to_DOT(&mut  self) -> String {
        let mut g = "digraph G { \n".to_string();
        for edge in &self.graph {
            g.push_str(edge.0.as_str());
            g.push_str(" -> ");
            g.push_str(edge.1.as_str());
            g.push_str(";\n");
        }
        g.push_str("}");
        g

    }

    pub fn graph_to_file(&mut self) {
        let DOT_graph = self.graph_to_DOT();
        let g: Graph = parse(DOT_graph.as_str()).unwrap();
        println!("{:?}", exec(g, &mut PrinterContext::default(), vec![
            CommandArg::Format(Format::Svg),
            CommandArg::Output("graph.svg".to_string())
        ]).err());
    }
    /*
    fn search_parents(&mut self, function_name: String) -> HashSet<String>{
        let mut result: HashSet<String> = HashSet::new();
        let mut doc_symbol_vec: Vec<DocumentSymbol> = Vec::new();
        let document = self.lang_server.document_open("/criu/fsnotify.c").unwrap();
        let doc_symbol = self.lang_server.document_symbol(&document).unwrap();

        match doc_symbol.clone() {
            Some(DocumentSymbolResponse::Flat(_)) => {
                println!("unsupported symbols found");
            },
            Some(DocumentSymbolResponse::Nested(doc_symbols)) => {
                for symbol in doc_symbols {
                    //println!("1{:?}", symbol.clone());
                    if symbol.kind == lsp_types::SymbolKind::Function {
                        println!("2{:?}", symbol.clone());
                        if !symbol.children.is_none() {
                            println!("3{:?}", symbol.children.clone());
                            let children = symbol.children.clone().unwrap();
                            for child in children {
                                if child.name == function_name {
                                    doc_symbol_vec[0] = symbol.clone();
                                    break;
                                }
                            }
                        }
                    }
                }
            },
            None => {
                println!("no symbols found");
            }
        }

        if doc_symbol_vec.len() != 0 {
            let doc_symbol = doc_symbol_vec[0].clone();
            for parent in doc_symbol.children.unwrap() {
                result.insert(parent.name);
            }
        }

        result
    }

    fn search_children(&mut self, function_name: String) -> HashSet<String>{
        let mut result: HashSet<String> = HashSet::new();
        let mut doc_symbol_vec: Vec<DocumentSymbol> = Vec::new();
        let document = self.lang_server.document_open("/criu/fsnotify.c").unwrap();
        println!("{:?}", document);
        let doc_symbol = self.lang_server.document_symbol(&document).unwrap();

        match doc_symbol.clone() {
            Some(DocumentSymbolResponse::Flat(_)) => {
                println!("unsupported symbols found");
            },
            Some(DocumentSymbolResponse::Nested(doc_symbols)) => {
                for symbol in doc_symbols {
                    println!("{:?}", symbol);
                    if symbol.name == function_name {
                        doc_symbol_vec[0] = symbol;
                    }

                    break;

                }
            },
            None => {
                println!("no symbols found");
            }
        }

        if doc_symbol_vec.len() != 0 {
            let doc_symbol = doc_symbol_vec[0].clone();
            for child in doc_symbol.children.unwrap() {
                result.insert(child.name);
            }
        }

        result
    }*/
}