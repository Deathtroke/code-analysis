use pest::Parser;
use pest_derive::Parser;
use pest::iterators::Pair;
use std::borrow::Borrow;
use std::collections::HashMap;
use super::*;

#[derive(Parser)]
#[grammar = "grammar.pest"]
struct MyParser;

pub struct parser {
    pub map :HashMap<String, String>
}

impl parser {
    pub fn parse_grammar(&self, input: String) -> Pair<Rule> {
        let pair = MyParser::parse(Rule::command, input.to_owned().as_str())
            .expect("unsuccessful parse")
            .next().unwrap();
        pair
    }

    pub fn parse(&mut self, input: String) {
        let pair = MyParser::parse(Rule::command, input.to_owned().as_str())
            .expect("unsuccessful parse")
            .next().unwrap();
        self.parse_command(pair);
    }

    fn parse_command(&mut self, pair: Pair<Rule>) -> Vec<String> {
        let mut function_names: Vec<String> = Vec::new();
        let mut overwrite_name : String = "".to_string();
        let mut search_parent = false; //false = search for child | true = search for parents
        for inner_pair in pair.to_owned().into_inner() {
            match inner_pair.as_rule() {
                Rule::command_type => {
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
                    function_names[0] = inner_pair.to_string();
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
        for name in function_names {
            if search_parent {
                function_names.append(&mut self.search_parent(name, overwrite_name));
            } else {
                function_names.append(&mut self.search_child(name, overwrite_name));
            }
        }
        function_names
    }

    fn search_parent(&mut self, search_target: String, overwrite_name: String)  -> Vec<String>{
        let parents :Vec<String> = searcher::search_parents(); //TODO this is only a dummy function
        for parent in parents {
            if overwrite_name == "" {
                self.map.insert(search_target.to_owned(), parent);
            } else {
                self.map.insert(overwrite_name.to_owned(), parent);
            }

        }
        parents
    }

    fn search_child(&mut self, search_target: String, overwrite_name: String)  -> Vec<String>{
        let children :Vec<String> = searcher::search_children(); //TODO this is only a dummy function
        for child in children {
            if overwrite_name == "" {
                self.map.insert(search_target.to_string().to_owned(), child);
            } else {
                self.map.insert(overwrite_name.to_string().to_owned(), child);
            }

        }
        children
    }

    fn parse_function(&mut self, pair: Pair<Rule>)  -> Vec<String>{
        let mut function_names: Vec<String> = Vec::new();
        let mut i = 0;
        for functions_pair in pair.into_inner() {
            match functions_pair.as_rule() {
                Rule::extra_command => {
                    //filter out {, } and whitespaces so only the next command can
                    for extra_command_pair in functions_pair.into_inner() {
                        match extra_command_pair.as_rule() {
                            Rule::extra_command => {
                                self.parse_command(extra_command_pair);
                            }
                            _ => {}
                        }
                    }
                }
                Rule::function_name => {
                    function_names[i] = functions_pair.to_string();
                    i += 1;
                }
                _ => {}
            }
        }
        function_names
    }
}