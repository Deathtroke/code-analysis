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

use crate::searcher::{FunctionEdge, LSPInterface, MatchFunctionEdge};

#[derive(Parser)]
#[grammar = "grammar.pest"]
struct MyParser;

pub struct parser {
    pub graph : graph::Graph,
    lang_server : searcher::LSPServer,
    files_in_project: Vec<String>,
    project_path: String,
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
        let p = parser{
            graph: graph::Graph {
                edges: HashSet::new(),
            },
            lang_server: searcher::LSPServer::new(project_path.clone()),
            files_in_project: get_all_files_in_project(project_path.clone(), project_path.clone()),
            project_path,
            //global_vars:HashSet::new(),
            //global_filter:HashSet::new()
        };
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
*/

    fn parse_statement (&mut self, pair: Pair<Rule>, mut parents: HashSet<String>) -> HashSet<String> {
        let mut parent_filter: Vec<HashMap<FilterName, String>> = Vec::new();
        let mut child_names: HashSet<String> = HashSet::new();
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
        if  parent_filter.len() > 0 {
            let parent_names = self.find_func_name(parent_filter);
            for mut parent in parent_names {
                if child_names.len() > 0 {
                    for child in child_names.clone() {
                        if self.paren_child_exists(parent.get_func_name(), child) {
                            parents.insert(parent.get_func_name());
                        }
                    }
                } else {
                    if do_search {
                        let children = self.search_child(parent.get_func_name());
                        if children.len() > 0 {
                            parents.insert(parent.get_func_name());
                        }
                    } else {
                        parents.insert(parent.get_func_name());
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

    fn parse_verb(&mut self, pair: Pair<Rule>) -> HashMap<FilterName, String>{
        let mut filter: HashMap<FilterName, String> = HashMap::new();
        for inner_pair in pair.to_owned().into_inner() {
            match inner_pair.as_rule() {
                Rule::predefined_ident => {
                    filter = self.parse_predefined_ident(inner_pair, filter);
                }
                Rule::ident => {
                    filter.insert(FilterName::Function, inner_pair.as_str().to_string());
                }
                _ => {}
            }
        }

        filter
    }

    fn parse_predefined_ident(&mut self, pair: Pair<Rule>, mut filter: HashMap<FilterName, String>) -> HashMap<FilterName, String> {
        let mut predefined_identifier_text = "";
        for inner_pair in pair.to_owned().into_inner() {
            match inner_pair.as_rule() {
                Rule::ident => {
                    predefined_identifier_text = inner_pair.as_str();
                }
                Rule::define_options => {
                    if predefined_identifier_text =="filter" {
                        let filter_option = self.parse_define_options(inner_pair);
                        if filter_option.is_some(){
                            let filter_option_unwrap = filter_option.unwrap();
                            filter.insert(filter_option_unwrap.0, filter_option_unwrap.1);
                        }
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

    fn search_all_connections_filter(&mut self, parent_filter: HashMap<String, String>, child_filter: HashMap<String, String>) -> HashSet<(String, String)> {
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
                        new_children = self.lang_server.search_parent_single_document_filter(func_filter_c.clone(), parent_filter.clone(), file_path.as_str());
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
                        new_children = self.lang_server.search_child_single_document_filter(func_filter_p.clone(), child_filter.clone(), file_path.as_str());
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
}

#[cfg(not(test))]
impl searcher::LSPInterface for parser {
    fn search_parent(&mut self, search_target: String)  -> HashSet<String>{
        let mut parent_filter:HashMap <String, String> = HashMap::new();
        parent_filter.insert("function".to_string(), ".".to_string());
        let mut child_filter:HashMap <String, String> = HashMap::new();
        child_filter.insert("function".to_string(),search_target.clone());

        //let parents:HashSet<String> = self.search_all_parents(search_target.clone());
        let connection :HashSet<(String, String)> = self.search_all_connections_filter(parent_filter, child_filter);

        let mut parents :HashSet<String> = HashSet::new();
        for parent in connection.clone() {
            self.graph.insert_edge(None, parent.0.clone(), search_target.to_string());
            parents.insert(parent.0);
        }
        parents
    }

    fn search_child(&mut self, search_target: String)  -> HashSet<String>{
        let mut parent_filter:HashMap <String, String> = HashMap::new();
        parent_filter.insert("function".to_string(), search_target.clone());
        let mut child_filter:HashMap <String, String> = HashMap::new();
        child_filter.insert("function".to_string(),".".to_string());

        let connection :HashSet<(String, String)> = self.search_all_connections_filter(parent_filter, child_filter);

        let mut children :HashSet<String> = HashSet::new();
        for child in connection.clone() {
            self.graph.insert_edge(None, search_target.to_string(), child.1.clone());
            children.insert(child.1);
        }
        children
    }

    fn paren_child_exists(&mut self, parent: String, child: String) -> bool{
        let result = self.search_child(parent.clone()).contains(child.as_str());

        if result {
            self.graph.insert_edge(None, parent, child);
        }

        result
    }

    fn search_connection_filter(&mut self, parent_filter: HashMap<String, String>, child_filter: HashMap<String, String>)  -> HashSet<(String, String)>{
        let parents:HashSet<(String, String)> = self.search_all_connections_filter(parent_filter.clone(), child_filter.clone());

        for parent in parents.clone() {
            //self.graph.insert((parent, search_target.clone()));

        }
        parents
    }

    fn find_func_name(&mut self, filter: Vec<HashMap<FilterName, String>>) -> HashSet<searcher::FunctionEdge>{
        let mut func_names :HashSet<searcher::FunctionEdge> = HashSet::new();
        for f in filter{
            let mut file_filter = Regex::new(".").unwrap();
            if f.contains_key(&FilterName::File){
                let regex = f.get(&FilterName::File).unwrap();
                file_filter = Regex::new(regex.clone().as_str()).unwrap();
            }

            let mut function_filter = Regex::new(".").unwrap();
            if f.contains_key(&FilterName::Function){
                let regex = f.get(&FilterName::Function).unwrap();
                function_filter = Regex::new(regex.as_str()).unwrap();
            }
            for file_path in self.files_in_project.clone(){
                if file_filter.is_match(file_path.as_str()) {
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

                    let need_lsp = function_filter.is_match(s.as_str());

                    if need_lsp
                    {

                        let names = self.lang_server.find_functions_in_doc(function_filter.clone(), file_path.as_str());
                        for name in names {
                            func_names.insert( FunctionEdge{function_name: name.clone(), document: file_path.clone()});
                        }
                    }
                }
            }
        }

        func_names
    }
}

#[cfg(test)]
mod parser_test;