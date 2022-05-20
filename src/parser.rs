use std::any::Any;
use pest::Parser;
use pest_derive::Parser;
use pest::iterators::Pair;
use std::borrow::Borrow;
use std::collections::{HashMap, HashSet};
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
use json::JsonValue::String as OtherString;
use juniper::GraphQLType;
use regex::Regex;
use stderrlog::new;

use crate::lang_server::LanguageServer;

#[derive(Parser)]
#[grammar = "grammar.pest"]
struct MyParser;

pub struct parser {
    pub graph :HashSet<(String, String)>,
    lang_server : Box<dyn LanguageServer>,
    files_in_project: Vec<String>,
    project_path: String,
    //global_vars :HashSet<(String, HashSet<(String, String)>)>,
    //global_filter :HashSet<(String, String)>
}

pub fn parse_grammar(input: &str) -> Pair<Rule> {
    let pair = MyParser::parse(Rule::query, input)
        .expect("unsuccessful parse")
        .next().unwrap();
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
            graph:HashSet::new(),
            lang_server: lang_server::LanguageServerLauncher::new()
                .server("/usr/bin/clangd".to_owned())
                //.server("/Users/hannes.boerner/Documents/clangd_14.0.3/bin/clangd-indexer".to_owned())
                .project(project_path.to_owned())
                //.languages(language_list)
                .launch()
                .expect("Failed to spawn clangd"),
            files_in_project: get_all_files_in_project(project_path.clone(), project_path.clone()),
            project_path,
            //global_vars:HashSet::new(),
            //global_filter:HashSet::new()
        };
        //println!("init {:?}", p.lang_server.initialize());
        p.lang_server.initialize();
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

    fn parse_verb(&mut self, pair: Pair<Rule>) -> String{
        let mut name = ".";
        for inner_pair in pair.to_owned().into_inner() {
            match inner_pair.as_rule() {
                Rule::predefined_ident => {

                }
                Rule::ident => {
                    name = inner_pair.as_str();
                    name = name.strip_prefix("@").unwrap();
                    while name.chars().last().unwrap() == ' '{
                        name = name.strip_suffix(" ").unwrap();
                    }
                }
                _ => {}
            }
        }

        name.to_string()
    }

    fn parse_predefined_ident(&mut self, pair: Pair<Rule>, mut filter: HashSet<(String, String)>){
        let mut predefined_identifier_text = "";
        for inner_pair in pair.to_owned().into_inner() {
            match inner_pair.as_rule() {
                Rule::ident => {
                    predefined_identifier_text = inner_pair.as_str();
                }
                Rule::define_options => {
                    if predefined_identifier_text =="filter" {
                        filter.insert(self.parse_define_options(inner_pair));
                    }
                }
                _ => {}
            }
        }
    }

    fn parse_define_options(&mut self, pair: Pair<Rule>) -> (String, String) {
        let mut attribute :String = String::new();
        let mut value:String = String::new();
        for inner_pair in pair.to_owned().into_inner() {
            match inner_pair.as_rule() {
                Rule::ident => {
                    attribute = inner_pair.to_string();
                }
                Rule::regex => {
                    value = inner_pair.to_string();
                }
                _ => {}
            }
        }
        (attribute, value)
    }



    fn search_parent(&mut self, search_target: String)  -> HashSet<String>{
        let mut parent_filter:HashMap <String, String> = HashMap::new();
        parent_filter.insert("function".to_string(), ".".to_string());
        let mut child_filter:HashMap <String, String> = HashMap::new();
        child_filter.insert("function".to_string(),search_target.clone());
        #[cfg(test)]
            let connection :HashSet<(String, String)> = HashSet::from([("parent1".to_string(), "".to_string()), ("parent2".to_string(), "".to_string())]);
        #[cfg(not(test))]
            let connection :HashSet<(String, String)> = self.search_all_connections_filter(parent_filter, child_filter);

        println!("{:?}", connection);

        let mut parents: HashSet<String> = HashSet::new();
        for parent in connection{
            parents.insert(parent.0);
        }

        for parent in parents.clone() {
            self.graph.insert((parent, search_target.clone()));

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
                new_parents = self.search_parent_single_document(search_target.clone(), file_path.as_str());
                println!("{:?}", new_parents);
            }
            for parent in new_parents {
                parents.insert(parent);
            }
            //thread::sleep(time::Duration::from_secs(1));
            //self.lang_server.
        }
        parents
    }

    fn search_child(&mut self, search_target: String)  -> HashSet<String>{
        #[cfg(test)]
            let mut children :HashSet<String> = HashSet::from(["child1".to_string(), "child2".to_string()]);
        #[cfg(not(test))]
            let mut children:HashSet<String> = self.search_all_children(search_target.clone());

        for child in children.clone() {
            self.graph.insert((search_target.clone(), child));
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
            self.graph.insert((parent.clone(), child.clone()));
        }

        result
    }

    fn search_connection_filter(&mut self, mut parent_filter: HashMap<String, String>, mut child_filter: HashMap<String, String>)  -> HashSet<(String, String)>{
        #[cfg(test)]
            let parents :HashSet<(String, String)> = HashSet::from([("parent1".to_string(), "parent2".to_string()), ("parent2".to_string(), "parent1".to_string())]);
        #[cfg(not(test))]
            let parents:HashSet<(String, String)> = self.search_all_connections_filter(parent_filter.clone(), child_filter.clone());

        for parent in parents.clone() {
            //self.graph.insert((parent, search_target.clone()));

        }
        parents
    }

    fn search_all_connections_filter(&mut self, mut parent_filter: HashMap<String, String>, mut child_filter: HashMap<String, String>) -> HashSet<(String, String)> {
        let mut connections:HashSet<(String, String)> = HashSet::new();

        let mut file_filter_p= Regex::new(".").unwrap(); //any
        if parent_filter.contains_key("file") {
            file_filter_p = Regex::new(parent_filter.get("file").unwrap().as_str()).unwrap();
        }
        let mut func_filter_p= Regex::new(".").unwrap(); //any
        if parent_filter.contains_key("function") {
            func_filter_p = Regex::new(parent_filter.get("function").unwrap().as_str()).unwrap();
        }

        let mut file_filter_c= Regex::new(".").unwrap(); //any
        if child_filter.contains_key("file") {
            file_filter_c = Regex::new(child_filter.get("file").unwrap().as_str()).unwrap();
        }
        let mut func_filter_c= Regex::new(".").unwrap(); //any
        if child_filter.contains_key("function") {
            func_filter_c = Regex::new(child_filter.get("function").unwrap().as_str()).unwrap();
        }

        if (file_filter_p.as_str() == ".") && (func_filter_p.as_str() == ".") {

            //println!("child {}",self.files_in_project.clone().len());
            for file_path in self.files_in_project.clone(){
                if file_filter_c.is_match(file_path.as_str()) {
                    let path = self.project_path.clone() + "/" + file_path.as_str();
                    let mut file = match File::open(&path) {
                        Err(why) => panic!("could not open: {}", why),
                        Ok(file) => file
                    };
                    let mut s = String::new();
                    match file.read_to_string(&mut s) {
                        Err(why) => panic!("could not read: {}", why),
                        Ok(_) => {}
                    }

                    let mut new_children = HashSet::new();
                    let need_lsp = func_filter_c.is_match(s.as_str());
                    //println!("{}", need_lsp);
                    if need_lsp
                    {
                        //println!("{}, {}", file_path, search_target.clone());
                        new_children = self.search_parent_single_document_filter(func_filter_c.clone(), parent_filter.clone(), file_path.as_str());
                        //println!("{:?}", new_children);
                    }
                    for child in new_children {
                        connections.insert(child);
                    }
                    //thread::sleep(time::Duration::from_secs(1));
                }
            }
        } else {

            //println!("child {}",self.files_in_project.clone().len());
            for file_path in self.files_in_project.clone(){
                if file_filter_p.is_match(file_path.as_str()) {
                    let path = self.project_path.clone() + "/" + file_path.as_str();
                    let mut file = match File::open(&path) {
                        Err(why) => panic!("could not open: {}", why),
                        Ok(file) => file
                    };
                    let mut s = String::new();
                    match file.read_to_string(&mut s) {
                        Err(why) => panic!("could not read: {}", why),
                        Ok(_) => {}
                    }

                    let mut new_children = HashSet::new();
                    let need_lsp = func_filter_p.is_match(s.as_str());
                    //println!("{}", need_lsp);
                    if need_lsp
                    {
                        //println!("{}, {}", file_path, search_target.clone());
                        new_children = self.search_child_single_document_filter(func_filter_p.clone(), child_filter.clone(), file_path.as_str());
                        //println!("{:?}", new_children);
                    }
                    for child in new_children {
                        connections.insert(child);
                    }
                    //thread::sleep(time::Duration::from_secs(1));
                }
            }
        }
        connections
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

    pub fn graph_to_file(&mut self, output_file: String) {
        let DOT_graph = self.graph_to_DOT();
        let g: Graph = parse(DOT_graph.as_str()).unwrap();
        println!("{:?}", exec(g, &mut PrinterContext::default(), vec![
            CommandArg::Format(Format::Svg),
            CommandArg::Output(output_file.clone())
        ]).err());
    }



    fn search_parent_single_document(&mut self, function_name: String, document_name: &str) -> HashSet<String> {
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
                        let incoming_calls = self.lang_server.call_hierarchy_item_incoming(prep_call_hierarchy.unwrap().unwrap()[0].clone());
                        for incoming_call in incoming_calls.unwrap().unwrap() {
                            result.insert(incoming_call.from.name.to_string());
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

    fn search_child_single_document_filter(&mut self, func_filter: Regex, mut child_filter: HashMap<String, String>, document_name: &str) -> HashSet<(String, String)> {
        let mut result: HashSet<(String, String)> = HashSet::new();
        let document = self.lang_server.document_open(document_name).unwrap();

        let mut file_filter_c= Regex::new(".").unwrap(); //any
        if child_filter.contains_key("file") {
            file_filter_c = Regex::new(child_filter.get("file").unwrap().as_str()).unwrap();
        }
        let mut func_filter_c= Regex::new(".").unwrap(); //any
        if child_filter.contains_key("function") {
            func_filter_c = Regex::new(child_filter.get("function").unwrap().as_str()).unwrap();
        }

        let doc_symbol = self.lang_server.document_symbol(&document).unwrap();

        match doc_symbol {
            Some(DocumentSymbolResponse::Flat(_)) => {
                println!("unsupported symbols found");
            },
            Some(DocumentSymbolResponse::Nested(doc_symbols)) => {
                for symbol in doc_symbols {
                    if symbol.kind == SymbolKind::FUNCTION {
                        let func_name = symbol.name;
                        //println!("func {}", func_name);
                        if func_filter.is_match(func_name.as_str()) {
                            let prep_call_hierarchy = self.lang_server.call_hierarchy_item(&document, symbol.range.start);
                            let call_hierarchy_array = prep_call_hierarchy.unwrap().unwrap();
                            if call_hierarchy_array.len() > 0 {
                                let outgoing_calls = self.lang_server.call_hierarchy_item_outgoing(call_hierarchy_array[0].clone());
                                for outgoing_call in outgoing_calls.unwrap().unwrap() {
                                    if func_filter_c.is_match(outgoing_call.to.name.as_str()) &&
                                        file_filter_c.is_match(outgoing_call.to.uri.as_str()) {
                                        result.insert((func_name.clone(), outgoing_call.to.name.to_string()));
                                    }
                                }
                                break;
                            }
                        }
                    }
                }
            },
            None => {
                println!("no symbols found");
            }
        }

        result
    }

    fn search_parent_single_document_filter(&mut self, func_filter: Regex, mut parent_filter: HashMap<String, String>, document_name: &str) -> HashSet<(String, String)> {
        let mut result: HashSet<(String, String)> = HashSet::new();
        let document = self.lang_server.document_open(document_name).unwrap();

        let mut file_filter_c= Regex::new(".").unwrap(); //any
        if parent_filter.contains_key("file") {
            file_filter_c = Regex::new(parent_filter.get("file").unwrap().as_str()).unwrap();
        }
        let mut func_filter_c= Regex::new(".").unwrap(); //any
        if parent_filter.contains_key("function") {
            func_filter_c = Regex::new(parent_filter.get("function").unwrap().as_str()).unwrap();
        }

        let doc_symbol = self.lang_server.document_symbol(&document).unwrap();

        match doc_symbol {
            Some(DocumentSymbolResponse::Flat(_)) => {
                println!("unsupported symbols found");
            },
            Some(DocumentSymbolResponse::Nested(doc_symbols)) => {
                for symbol in doc_symbols {
                    if symbol.kind == SymbolKind::FUNCTION {
                        let func_name = symbol.name;
                        //println!("{}", func_name);
                        if func_filter.is_match(func_name.as_str()) {
                            let prep_call_hierarchy = self.lang_server.call_hierarchy_item(&document, symbol.range.start);
                            let call_hierarchy_array = prep_call_hierarchy.unwrap().unwrap();
                            if call_hierarchy_array.len() > 0 {
                                let incoming_calls = self.lang_server.call_hierarchy_item_incoming(call_hierarchy_array[0].clone());
                                for incoming_call in incoming_calls.unwrap().unwrap() {
                                    if func_filter_c.is_match(incoming_call.from.name.as_str()) &&
                                        file_filter_c.is_match(incoming_call.from.uri.as_str()) {
                                        result.insert((incoming_call.from.name.to_string(), func_name.clone()));
                                    }
                                }
                                break;
                            }
                        }
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