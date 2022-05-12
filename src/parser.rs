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
    lang_server : Box<dyn LanguageServer>
}

pub fn parse_grammar(input: &str) -> Pair<Rule> {
    let pair = MyParser::parse(Rule::command, input)
        .expect("unsuccessful parse")
        .next().unwrap();
    pair
}
/*
impl parser {
    pub fn new() -> parser {
        let mut p = parser{
            graph:HashSet::new(),
            lang_server: lang_server::LanguageServerLauncher::new()
                .server("/usr/bin/clangd".to_owned())
                .project("/Users/hannes.boerner/Downloads/criu-criu-dev".to_owned())
                //.languages(language_list)
                .launch()
                .expect("Failed to spawn clangd")
        };
        p.lang_server.initialize();
        p
    }

    pub fn parse(&mut self, input: &str) -> HashSet<String>{
        let pair = parse_grammar(input);
        self.parse_command(pair)
    }

    fn parse_command(&mut self, pair: Pair<Rule>) -> HashSet<String> {
        let mut function_names: HashSet<String> = HashSet::new();
        let mut overwrite_name : String = "".to_string();
        let mut search_parent = false; //false = search for child | true = search for parents
        for inner_pair in pair.to_owned().into_inner() {
            match inner_pair.as_rule() {
                Rule::requests => {
                    match inner_pair.as_str() {
                        "parent" => {
                            search_parent = true;
                        }
                        "child" => {
                            search_parent = false;
                        },
                        _ => println!("command not implemented"),
                    };
                }
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
                _ => {}
            }
        }
        let mut output: HashSet<String> = HashSet::new();
        for name in function_names {
            let result;
            if search_parent {
                result = self.search_parent(name.clone(), overwrite_name.clone());
            } else {
                result = self.search_child(name.clone(), overwrite_name.clone());
            }
            for item in result {
                output.insert(item);
            }
        }
        output
    }

    fn search_parent(&mut self, search_target: String, overwrite_name: String)  -> HashSet<String>{
        #[cfg(test)]
            let parents :HashSet<String> = HashSet::from(["parent1".to_string(), "parent2".to_string()]);
        #[cfg(not(test))]
            let parents :HashSet<String> = self.search_parents(search_target.clone());
        for parent in parents.clone() {
            if overwrite_name == "" {
                self.graph.insert((parent, search_target.clone()));
            } else {
                self.graph.insert((parent, overwrite_name.clone()));
            }

        }
        parents
    }

    fn search_child(&mut self, search_target: String, overwrite_name: String)  -> HashSet<String>{
        #[cfg(test)]
            let children :HashSet<String> = HashSet::from(["child1".to_string(), "child2".to_string()]);
        #[cfg(not(test))]
            let children :HashSet<String> = self.search_children(search_target.clone());
        for child in children.clone() {
            if overwrite_name == "" {
                self.graph.insert((search_target.clone(), child));
            } else {
                self.graph.insert((overwrite_name.clone(), child));
            }

        }
        children
    }

    fn parse_function(&mut self, pair: Pair<Rule>)  -> HashSet<String>{
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
    }

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
    }
}*/