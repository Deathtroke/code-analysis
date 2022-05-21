use std::any::Any;
use pest::Parser;
use pest_derive::Parser;
use pest::iterators::{Pair, Pairs};
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
use std::fs;
use std::fs::File;
use std::io::prelude::*;
use std::path::Path;

use std::{thread, time};
use json::Error;
use json::JsonValue::String as OtherString;
use juniper::GraphQLType;

use serde::ser::{Serialize, Serializer, SerializeStruct, SerializeSeq};

use crate::lang_server::LanguageServer;

#[derive(Parser)]
#[grammar = "grammar.pest"]
struct MyParser;

pub struct parser {
    pub graph :Graph,
    lang_server : Box<dyn LanguageServer>,
    files_in_project: Vec<String>,
    project_path: String,
    //global_vars :HashSet<(String, HashSet<(String, String)>)>,
    //global_filter :HashSet<(String, String)>
}

#[derive(Eq, Hash, PartialEq)]
pub struct Edge {
    edge_properties: Option<String>,
    node_from: String,
    node_to: String,
}

pub struct Graph {
    edges: HashSet<Edge>,
}

impl Serialize for Edge {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
        let mut s = serializer.serialize_struct("Edge", 3)?;
        s.serialize_field("edge_properties", &self.edge_properties)?;
        s.serialize_field("from_node", &self.node_from)?;
        s.serialize_field("to_node", &self.node_to)?;
        s.end()
    }
}

impl Serialize for Graph {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
        let mut s = serializer.serialize_seq( Some(self.edges.len()))?;
        for edge in &self.edges {
            s.serialize_element(&edge);
        }
        s.end()
    }
}


pub fn parse_grammar(input: &str) -> Result<Pairs<Rule>, pest::error::Error<Rule>> {
    let pair = MyParser::parse(Rule::query, input);
        //.expect("unsuccessful parse")
        //.next();
    pair
}

fn get_all_files_in_project(dir: String, project_path: String) -> Vec<String>{
    let mut files :Vec<String> = Vec::new();
    let paths = fs::read_dir(dir.clone()).unwrap();

    for path in paths {
        let path_str = path.as_ref().unwrap().path().to_str().unwrap().to_string();
        if path.as_ref().unwrap().metadata().unwrap().is_dir() {
            let mut subfolder = get_all_files_in_project(path_str, project_path.clone());
            files.append(&mut subfolder);
        } else {
            if path_str.ends_with(".cpp") || path_str.ends_with(".c"){
                files.push(path_str.replace(&(project_path.clone().as_str().to_owned() + "/"),""));
            }
        }
    }
    files
}

impl parser {
    pub fn new(project_path: String) -> parser {
        let mut p = parser{
            graph: Graph {
                edges: HashSet::new(),
            },
            lang_server: lang_server::LanguageServerLauncher::new()
                .server("/usr/bin/clangd".to_owned())
                .project(project_path.to_owned())
                //.languages(language_list)
                .launch()
                .expect("Failed to spawn clangd"),
            files_in_project: get_all_files_in_project(project_path.clone(), project_path.clone()),
            project_path,
            //global_vars:HashSet::new(),
            //global_filter:HashSet::new()
        };
        p.lang_server.initialize();
        p
    }

    pub fn parse(&mut self, input: &str) -> HashSet<String>{
        let pair = parse_grammar(input);
        if pair.is_ok() {
            self.parse_statements(pair.unwrap().next().unwrap())
        } else {
            println!("unable to parse input: {:?}", pair.err());
            HashSet::new()
        }
    }

    fn parse_statements(&mut self, pair: Pair<Rule>) -> HashSet<String> {
        let mut function_names: HashSet<String> = HashSet::new();
        //let mut overwrite_name : String = "".to_string();

        for inner_pair in pair.to_owned().into_inner() {
            match inner_pair.as_rule() {
                Rule::statement =>{
                    function_names = self.parse_statement(inner_pair,function_names.clone());
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
                    parent_names.insert(name.to_string());
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
            let parents:HashSet<String> = self.search_all_parents(search_target.clone());

        for parent in parents.clone() {
            let edge: Edge = Edge{ edge_properties: None, node_from: parent, node_to: search_target.clone()};
            self.graph.edges.insert(edge);

        }
        parents
    }
    fn search_all_parents(&mut self, search_target: String) -> HashSet<String> {
        let mut parents:HashSet<String> = HashSet::new();
        //println!("parent {}",self.files_in_project.clone().len());
        for file_path in self.files_in_project.clone(){
            let path = self.project_path.clone() + "/" + file_path.as_str();
            let mut file = match File::open(&path){
                Err(why) => panic!("could not open: {}", why),
                Ok(file) => file
            };
            let mut s = String::new();
            match file.read_to_string(&mut s){
                Err(why) => panic!("could not read: {}", why),
                Ok(_) => { }
            }

            let mut new_parents = HashSet::new();
            let need_lsp = s.contains(&search_target.clone());
            //println!("{}", need_lsp);
            if need_lsp
            {
                println!("{}, {}", file_path, search_target.clone());
                new_parents = self.search_parent_single_document(search_target.clone(), file_path.as_str()).unwrap();
                println!("{:?}", new_parents);
            }
            for parent in new_parents {
                parents.insert(parent);
            }
            //thread::sleep(time::Duration::from_secs(1));
        }
        parents
    }

    fn search_child(&mut self, search_target: String)  -> HashSet<String>{
        #[cfg(test)]
            let mut children :HashSet<String> = HashSet::from(["child1".to_string(), "child2".to_string()]);
        #[cfg(not(test))]
            let mut children:HashSet<String> = self.search_all_children(search_target.clone());

        for child in children.clone() {
            let edge: Edge = Edge{ edge_properties: None, node_from: search_target.clone(), node_to: child};
            self.graph.edges.insert(edge);
        }
        children
    }

    fn search_all_children(&mut self, search_target: String) -> HashSet<String> {
        let mut children:HashSet<String> = HashSet::new();
        //println!("child {}",self.files_in_project.clone().len());
        for file_path in self.files_in_project.clone(){
            let path = self.project_path.clone() + "/" + file_path.as_str();
            let mut file = match File::open(&path){
                Err(why) => panic!("could not open: {}", why),
                Ok(file) => file
            };
            let mut s = String::new();
            match file.read_to_string(&mut s){
                Err(why) => panic!("could not read: {}", why),
                Ok(_) => { }
            }

            let mut new_children = HashSet::new();
            let need_lsp = s.contains(&search_target.clone());
            //println!("{}", need_lsp);
            if need_lsp
            {
                //println!("{}, {}", file_path, search_target.clone());
                new_children = self.search_child_single_document(search_target.clone(), file_path.as_str());
                //println!("{:?}", new_children);
            }
            for child in new_children {
                children.insert(child);
            }
            //thread::sleep(time::Duration::from_secs(1));
        }
        children
    }

    fn paren_child_exists(&mut self, parent: String, child: String) -> bool{
        #[cfg(test)]
            let result = (parent == "parent1" || parent == "parent2") &&
            (child == "child1" || child == "child2") ;
        #[cfg(not(test))]
            let result = self.search_child(parent.clone()).contains(child.as_str());

        if result {
            let edge: Edge = Edge{ edge_properties: None, node_from: parent, node_to: child};
            self.graph.edges.insert(edge);
        }

        result
    }

    fn search_parent_single_document(&mut self, function_name: String, document_name: &str) -> Result<HashSet<String>,  lang_server::Error> {
        let mut result: Result<HashSet<String>, lang_server::Error> = Ok(HashSet::new());
        let document_res = self.lang_server.document_open(document_name);
        if document_res.is_ok(){
            let document = document_res.unwrap();

            let doc_symbol_res = self.lang_server.document_symbol(&document);
            if doc_symbol_res.is_ok(){
                let doc_symbol = doc_symbol_res.unwrap();

                match doc_symbol {
                    Some(DocumentSymbolResponse::Flat(_)) => {
                        println!("unsupported symbols found");
                    },
                    Some(DocumentSymbolResponse::Nested(doc_symbols)) => {
                        let mut children = HashSet::new();
                        for symbol in doc_symbols {
                            if symbol.name == function_name {
                                let prep_call_hierarchy_res = self.lang_server.call_hierarchy_item(&document, symbol.range.start);
                                if prep_call_hierarchy_res.is_ok(){
                                    let call_hierarchy_item = prep_call_hierarchy_res.unwrap().unwrap()[0].clone();

                                    let incoming_calls = self.lang_server.call_hierarchy_item_incoming(call_hierarchy_item);
                                    for incoming_call in incoming_calls.unwrap().unwrap() {
                                        children.insert(incoming_call.from.name.to_string());
                                    }
                                    break;
                                } else {
                                    result = Err(prep_call_hierarchy_res.err().unwrap());
                                    return result;
                                }
                            }
                        }
                        result = Ok(children);
                    },
                    None => {
                        println!("no symbols found");
                    }
                }
            } else {
                result = Err(doc_symbol_res.err().unwrap());
                return result;
            }
        } else {
            result = Err(document_res.err().unwrap());
            return result;
        }

        result
    }

    fn search_child_single_document(&mut self, function_name: String, document_name: &str) -> HashSet<String> {
        let mut result: HashSet<String> = HashSet::new();
        let document = self.lang_server.document_open(document_name).unwrap();

        let doc_symbol = self.lang_server.document_symbol(&document).unwrap();

        match doc_symbol {
            Some(DocumentSymbolResponse::Flat(_)) => {
                println!("unsupported symbols found");
            },
            Some(DocumentSymbolResponse::Nested(doc_symbols)) => {
                for symbol in doc_symbols {
                    if symbol.name == function_name {
                        let prep_call_hierarchy = self.lang_server.call_hierarchy_item(&document, symbol.range.start);
                        let outgoing_calls = self.lang_server.call_hierarchy_item_outgoing(prep_call_hierarchy.unwrap().unwrap()[0].clone());
                        for outgoing_call in outgoing_calls.unwrap().unwrap() {
                            result.insert(outgoing_call.to.name.to_string());
                        }
                        break;
                    }

                }
            },
            None => {
                println!("no symbols found");
            }
        }

        result
    }
}

impl Graph {
    pub fn graph_to_DOT(&mut  self) -> String {
        let mut g = "digraph G { \n".to_string();
        for edge in &self.edges {
            g.push_str(edge.node_from.as_str());
            g.push_str(" -> ");
            g.push_str(edge.node_to.as_str());
            g.push_str(";\n");
        }
        g.push_str("}");
        g

    }

    pub fn graph_to_file(&mut self, output_file: String) {
        let DOT_graph = self.graph_to_DOT();
        let g: dot_structures::Graph = parse(DOT_graph.as_str()).unwrap();
        println!("{:?}", exec(g, &mut PrinterContext::default(), vec![
            CommandArg::Format(Format::Svg),
            CommandArg::Output(output_file.clone())
        ]).err());
    }
}