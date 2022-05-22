use std::any::Any;
use pest::Parser;
use pest_derive::Parser;
use pest::iterators::{Pair, Pairs};
use std::borrow::Borrow;
use std::collections::HashSet;
use std::string::String;
use super::*;
use std::fs;
use std::fs::File;
use std::io::prelude::*;
use std::path::Path;
use anyhow;
use tabbycat;

use std::{thread, time};
use json::Error;
use json::JsonValue::String as OtherString;
use juniper::GraphQLType;

use crate::searcher::LSPInterface;

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
                println!("search in {}, function {}", file_path, search_target.clone());
                new_parents = self.lang_server.search_parent_single_document(search_target.clone(), file_path.as_str()).unwrap();
                println!("{:?}", new_parents);
            }
            for parent in new_parents {
                parents.insert(parent);
            }
            //thread::sleep(time::Duration::from_secs(1));
        }
        parents
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
                new_children = self.lang_server.search_child_single_document(search_target.clone(), file_path.as_str());
                //println!("{:?}", new_children);
            }
            for child in new_children {
                children.insert(child);
            }
            //thread::sleep(time::Duration::from_secs(1));
        }
        children
    }
}

#[cfg(not(test))]
impl searcher::LSPInterface for parser {
    fn search_parent(&mut self, search_target: String)  -> HashSet<String>{
        let parents:HashSet<String> = self.search_all_parents(search_target.clone());

        for parent in parents.clone() {
            self.graph.insert_edge(None, parent, search_target.to_string());
        }
        parents
    }

    fn search_child(&mut self, search_target: String)  -> HashSet<String>{
        let mut children:HashSet<String> = self.search_all_children(search_target.clone());

        for child in children.clone() {
            self.graph.insert_edge(None, search_target.to_string(), child);
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
}

#[cfg(test)]
mod parser_test;