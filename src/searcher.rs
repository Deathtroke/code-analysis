use crate::lang_server::LanguageServer;
use crate::{lang_server, parser};
use lsp_types::{DocumentSymbolResponse, SymbolKind};
use regex::Regex;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::fs::File;
use std::hash::{Hash, Hasher};
use std::io::prelude::*;

pub trait LSPServer {
    fn search_parent(&mut self, search_target: String) -> HashSet<String>;
    fn search_child(&mut self, search_target: String) -> HashSet<String>;
    fn search_connection_filter(
        &mut self,
        parent_filter: HashMap<String, String>,
        child_filter: HashMap<String, String>,
    ) -> HashSet<(String, String)>;
    fn find_func_name(
        &mut self,
        filter: Vec<HashMap<parser::FilterName, String>>,
    ) -> HashSet<FunctionNode>;
    fn search_child_single_document_filter(
        &mut self,
        func_filter: Regex,
        child_filter: HashMap<String, String>,
        document_name: &str,
    ) -> HashSet<(String, String)>;
    fn search_parent_single_document_filter(
        &mut self,
        func_filter: Regex,
        parent_filter: HashMap<String, String>,
        document_name: &str,
    ) -> HashSet<(String, String)>;
   fn find_link(&mut self, parent_name: String, child_name: String, document_name: &str) -> bool;
    fn find_functions_in_doc(&mut self, func_filter: Regex, document_name: &str)
        -> HashSet<String>;
}

fn get_all_files_in_project(dir: String, project_path: String) -> Vec<String> {
    let mut files: Vec<String> = Vec::new();
    let paths = fs::read_dir(dir.clone()).unwrap();

    for path in paths {
        let path_str = path.as_ref().unwrap().path().to_str().unwrap().to_string();
        if path.as_ref().unwrap().metadata().unwrap().is_dir() {
            let mut subfolder = get_all_files_in_project(path_str, project_path.clone());
            files.append(&mut subfolder);
        } else {
            if path_str.ends_with(".cpp") || path_str.ends_with(".c") {
                files.push(path_str.replace(&(project_path.clone().as_str().to_owned() + "/"), ""));
            }
        }
    }
    files
}

pub struct ClangdServer {
    pub lang_server: Box<dyn LanguageServer>,
    pub files_in_project: Vec<String>,
    pub project_path: String,
}

pub struct FunctionNode {
    pub function_name: String,
    pub document: String,
    pub match_strategy: Box<dyn MatchFunctionEdge>
}

impl Hash for FunctionNode {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.function_name.hash(state);
        self.document.hash(state);
    }
}

impl PartialEq for FunctionNode {
    fn eq(&self, other: &Self) -> bool {
        self.document == other.document && self.function_name == other.function_name
    }
}

impl Eq for FunctionNode {}

impl Clone for FunctionNode {
    fn clone(&self) -> Self {
        match self.match_strategy.get_implementation().as_str() {
            "ForcedEdge" => {
                let strategy = ForcedEdge { function_name: self.function_name.clone(), document: self.document.clone() };
                FunctionNode{
                    function_name: self.function_name.clone(),
                    document: self.function_name.clone(),
                    match_strategy: Box::new(strategy),
                }

            }
            "ParentChildEdge" => {
                let strategy = ParentChildEdge { function_name: self.function_name.clone(), document: self.document.clone() };
                FunctionNode{
                    function_name: self.function_name.clone(),
                    document: self.function_name.clone(),
                    match_strategy: Box::new(strategy),
                }
            }
            _ => {unimplemented!()}
        }
    }
}

pub trait MatchFunctionEdge {
    fn do_match(&mut self, match_target: FunctionNode, lsp_server: &mut Box<dyn LSPServer>) -> bool;
    fn get_implementation(&self) -> String;
}

#[derive(Hash, Eq, PartialEq, Debug, Clone)]
pub struct ForcedEdge{
    pub function_name: String,
    pub document: String,
}

pub struct ParentChildEdge {
    pub function_name: String,
    pub document: String,
}

impl MatchFunctionEdge for ForcedEdge {
    fn do_match(&mut self, match_target: FunctionNode, lsp_server: &mut Box<dyn LSPServer>) -> bool {
        #[allow(dead_code)]
        if false {match_target; lsp_server; unimplemented!()}
        true
    }
    fn get_implementation(&self) -> String {
        "ForcedEdge".to_string()
    }

}

impl MatchFunctionEdge for ParentChildEdge {
    fn do_match(&mut self, match_target: FunctionNode, lsp_server: &mut Box<dyn LSPServer>) -> bool {
        lsp_server.find_link(self.function_name.clone(), match_target.function_name, self.document.as_str())
    }
    fn get_implementation(&self) -> String{
        "ParentChildEdge".to_string()
    }

}

impl ClangdServer {
    pub fn new(project_path: String) -> Box<dyn LSPServer> {
        let mut lsp_server = Self {
            lang_server: lang_server::LanguageServerLauncher::new()
                .server("/usr/bin/clangd".to_owned())
                .project(project_path.to_owned())
                //.languages(language_list)
                .launch()
                .expect("Failed to spawn clangd"),
            files_in_project: get_all_files_in_project(project_path.clone(), project_path.clone()),
            project_path,
        };
        let res = lsp_server.lang_server.initialize();
        if res.is_err() {
            println!("LSP server didn't initialize: {:?}", res.err())
        }
        Box::new(lsp_server)
    }
}

impl LSPServer for ClangdServer {
    fn search_parent(&mut self, search_target: String) -> HashSet<String> {
        let mut parent_filter: HashMap<String, String> = HashMap::new();
        parent_filter.insert("function".to_string(), ".".to_string());
        let mut child_filter: HashMap<String, String> = HashMap::new();
        child_filter.insert("function".to_string(), search_target.clone());

        //let parents:HashSet<String> = self.search_all_parents(search_target.clone());
        let connection: HashSet<(String, String)> =
            self.search_connection_filter(parent_filter, child_filter);

        let mut parents: HashSet<String> = HashSet::new();
        for parent in connection.clone() {
            //self.graph.insert_edge(None, parent.0.clone(), search_target.to_string());
            parents.insert(parent.0);
        }
        parents
    }

    fn search_child(&mut self, search_target: String) -> HashSet<String> {
        let mut parent_filter: HashMap<String, String> = HashMap::new();
        parent_filter.insert("function".to_string(), search_target.clone());
        let mut child_filter: HashMap<String, String> = HashMap::new();
        child_filter.insert("function".to_string(), ".".to_string());

        let connection: HashSet<(String, String)> =
            self.search_connection_filter(parent_filter, child_filter);

        let mut children: HashSet<String> = HashSet::new();
        for child in connection.clone() {
            //self.graph.insert_edge(None, search_target.to_string(), child.1.clone());
            children.insert(child.1);
        }
        children
    }

    fn search_connection_filter(
        &mut self,
        parent_filter: HashMap<String, String>,
        child_filter: HashMap<String, String>,
    ) -> HashSet<(String, String)> {
        let mut connections: HashSet<(String, String)> = HashSet::new();

        let mut file_filter_p = Regex::new(".").unwrap(); //any
        if parent_filter.contains_key("file") {
            file_filter_p = Regex::new(parent_filter.get("file").unwrap().as_str()).unwrap();
        }
        let mut func_filter_p = Regex::new(".").unwrap(); //any
        if parent_filter.contains_key("function") {
            func_filter_p = Regex::new(parent_filter.get("function").unwrap().as_str()).unwrap();
        }

        let mut file_filter_c = Regex::new(".").unwrap(); //any
        if child_filter.contains_key("file") {
            file_filter_c = Regex::new(child_filter.get("file").unwrap().as_str()).unwrap();
        }
        let mut func_filter_c = Regex::new(".").unwrap(); //any
        if child_filter.contains_key("function") {
            func_filter_c = Regex::new(child_filter.get("function").unwrap().as_str()).unwrap();
        }

        if (file_filter_p.as_str() == ".") && (func_filter_p.as_str() == ".") {
            for file_path in self.files_in_project.clone() {
                if file_filter_c.is_match(file_path.as_str()) {
                    let path = self.project_path.clone() + "/" + file_path.as_str();
                    let mut file = match File::open(&path) {
                        Err(why) => panic!("could not open: {}", why),
                        Ok(file) => file,
                    };
                    let mut s = String::new();
                    match file.read_to_string(&mut s) {
                        Err(why) => panic!("could not read: {}", why),
                        Ok(_) => {}
                    }

                    let mut new_children = HashSet::new();
                    let need_lsp = func_filter_c.is_match(s.as_str());
                    //println!("{}", need_lsp);
                    if need_lsp {
                        //println!("{}, {}", file_path, search_target.clone());
                        new_children = self.search_parent_single_document_filter(
                            func_filter_c.clone(),
                            parent_filter.clone(),
                            file_path.as_str(),
                        );
                        //println!("{:?}", new_children);
                    }
                    for child in new_children {
                        connections.insert(child);
                    }
                    //thread::sleep(time::Duration::from_secs(1));
                }
            }
        } else {
            for file_path in self.files_in_project.clone() {
                if file_filter_p.is_match(file_path.as_str()) {
                    let path = self.project_path.clone() + "/" + file_path.as_str();
                    let mut file = match File::open(&path) {
                        Err(why) => panic!("could not open: {}", why),
                        Ok(file) => file,
                    };
                    let mut s = String::new();
                    match file.read_to_string(&mut s) {
                        Err(why) => panic!("could not read: {}", why),
                        Ok(_) => {}
                    }

                    let mut new_children = HashSet::new();
                    let need_lsp = func_filter_p.is_match(s.as_str());
                    //println!("{}", need_lsp);
                    if need_lsp {
                        //println!("{}, {}", file_path, search_target.clone());
                        new_children = self.search_child_single_document_filter(
                            func_filter_p.clone(),
                            child_filter.clone(),
                            file_path.as_str(),
                        );
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

    fn find_func_name(
        &mut self,
        filter: Vec<HashMap<parser::FilterName, String>>,
    ) -> HashSet<FunctionNode> {
        let mut func_names: HashSet<FunctionNode> = HashSet::new();
        for f in filter {
            let mut file_filter = Regex::new(".").unwrap();
            if f.contains_key(&parser::FilterName::File) {
                let regex = f.get(&parser::FilterName::File).unwrap();
                file_filter = Regex::new(regex.clone().as_str()).unwrap();
            }

            let mut function_filter = Regex::new(".").unwrap();
            if f.contains_key(&parser::FilterName::Function) {
                let regex = f.get(&parser::FilterName::Function).unwrap();
                function_filter = Regex::new(regex.as_str()).unwrap();
            }
            for file_path in self.files_in_project.clone() {
                if file_filter.is_match(file_path.as_str()) {
                    let path = self.project_path.clone() + "/" + file_path.as_str();
                    let mut file = match File::open(&path) {
                        Err(why) => panic!("could not open: {}", why),
                        Ok(file) => file,
                    };
                    let mut s = String::new();
                    match file.read_to_string(&mut s) {
                        Err(why) => panic!("could not read: {}", why),
                        Ok(_) => {}
                    }

                    let need_lsp = function_filter.is_match(s.as_str());

                    if need_lsp {
                        let names =
                            self.find_functions_in_doc(function_filter.clone(), file_path.as_str());
                        for name in names {
                            let prent_child_edge = ParentChildEdge{
                                function_name: name.clone(),
                                document: file_path.clone()
                            };
                            func_names.insert( FunctionNode{function_name: name.clone(), document: file_path.clone(), match_strategy: Box::new(prent_child_edge)});
                        }
                    }
                }
            }
        }

        func_names
    }
    fn search_child_single_document_filter(
        &mut self,
        func_filter: Regex,
        child_filter: HashMap<String, String>,
        document_name: &str,
    ) -> HashSet<(String, String)> {
        let mut result: HashSet<(String, String)> = HashSet::new();
        let document = self.lang_server.document_open(document_name).unwrap();

        let mut file_filter_c = Regex::new(".").unwrap(); //any
        if child_filter.contains_key("file") {
            file_filter_c = Regex::new(child_filter.get("file").unwrap().as_str()).unwrap();
        }
        let mut func_filter_c = Regex::new(".").unwrap(); //any
        if child_filter.contains_key("function") {
            func_filter_c = Regex::new(child_filter.get("function").unwrap().as_str()).unwrap();
        }

        let doc_symbol = self.lang_server.document_symbol(&document).unwrap();

        match doc_symbol {
            Some(DocumentSymbolResponse::Flat(_)) => {
                println!("unsupported symbols found");
            }
            Some(DocumentSymbolResponse::Nested(doc_symbols)) => {
                for symbol in doc_symbols {
                    if symbol.kind == SymbolKind::FUNCTION {
                        let func_name = symbol.name;
                        //println!("func {}", func_name);
                        if func_filter.is_match(func_name.as_str()) {
                            let prep_call_hierarchy = self
                                .lang_server
                                .call_hierarchy_item(&document, symbol.range.start);
                            let call_hierarchy_array = prep_call_hierarchy.unwrap().unwrap();
                            if call_hierarchy_array.len() > 0 {
                                let outgoing_calls = self
                                    .lang_server
                                    .call_hierarchy_item_outgoing(call_hierarchy_array[0].clone());
                                for outgoing_call in outgoing_calls.unwrap().unwrap() {
                                    if func_filter_c.is_match(outgoing_call.to.name.as_str())
                                        && file_filter_c.is_match(outgoing_call.to.uri.as_str())
                                    {
                                        result.insert((
                                            func_name.clone(),
                                            outgoing_call.to.name.to_string(),
                                        ));
                                    }
                                }
                                break;
                            }
                        }
                    }
                }
            }
            None => {
                println!("no symbols found");
            }
        }

        result
    }

    fn search_parent_single_document_filter(
        &mut self,
        func_filter: Regex,
        parent_filter: HashMap<String, String>,
        document_name: &str,
    ) -> HashSet<(String, String)> {
        let mut result: HashSet<(String, String)> = HashSet::new();
        let document = self.lang_server.document_open(document_name).unwrap();

        let mut file_filter_c = Regex::new(".").unwrap(); //any
        if parent_filter.contains_key("file") {
            file_filter_c = Regex::new(parent_filter.get("file").unwrap().as_str()).unwrap();
        }
        let mut func_filter_c = Regex::new(".").unwrap(); //any
        if parent_filter.contains_key("function") {
            func_filter_c = Regex::new(parent_filter.get("function").unwrap().as_str()).unwrap();
        }

        let doc_symbol = self.lang_server.document_symbol(&document).unwrap();

        match doc_symbol {
            Some(DocumentSymbolResponse::Flat(_)) => {
                println!("unsupported symbols found");
            }
            Some(DocumentSymbolResponse::Nested(doc_symbols)) => {
                for symbol in doc_symbols {
                    if symbol.kind == SymbolKind::FUNCTION {
                        let func_name = symbol.name;
                        //println!("{}", func_name);
                        if func_filter.is_match(func_name.as_str()) {
                            let prep_call_hierarchy = self
                                .lang_server
                                .call_hierarchy_item(&document, symbol.range.start);
                            let call_hierarchy_array = prep_call_hierarchy.unwrap().unwrap();
                            if call_hierarchy_array.len() > 0 {
                                let incoming_calls = self
                                    .lang_server
                                    .call_hierarchy_item_incoming(call_hierarchy_array[0].clone());
                                for incoming_call in incoming_calls.unwrap().unwrap() {
                                    if func_filter_c.is_match(incoming_call.from.name.as_str())
                                        && file_filter_c.is_match(incoming_call.from.uri.as_str())
                                    {
                                        result.insert((
                                            incoming_call.from.name.to_string(),
                                            func_name.clone(),
                                        ));
                                    }
                                }
                            }
                        }
                    }
                }
            }
            None => {
                println!("no symbols found");
            }
        }

        result
    }

    fn find_link(&mut self, parent_name: String, child_name: String, document_name: &str) -> bool {
        let document_res = self.lang_server.document_open(document_name);
        if document_res.is_ok() {
            let document = document_res.unwrap();
            let doc_symbol = self.lang_server.document_symbol(&document).unwrap();

            match doc_symbol {
                Some(DocumentSymbolResponse::Flat(_)) => {
                    println!("unsupported symbols found");
                }
                Some(DocumentSymbolResponse::Nested(doc_symbols)) => {
                    for symbol in doc_symbols {
                        if symbol.kind == SymbolKind::FUNCTION {
                            let func_name = symbol.name;
                            //println!("{}", func_name);
                            if parent_name == func_name {
                                let prep_call_hierarchy = self
                                    .lang_server
                                    .call_hierarchy_item(&document, symbol.range.start);
                                let call_hierarchy_array = prep_call_hierarchy.unwrap().unwrap();
                                if call_hierarchy_array.len() > 0 {
                                    let outgoing_calls =
                                        self.lang_server.call_hierarchy_item_outgoing(
                                            call_hierarchy_array[0].clone(),
                                        );
                                    for outgoing_call in outgoing_calls.unwrap().unwrap() {
                                        if outgoing_call.to.name.as_str() == child_name {
                                            return true;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                None => {
                    println!("no symbols found");
                }
            }
        }
        false
    }

    fn find_functions_in_doc(
        &mut self,
        func_filter: Regex,
        document_name: &str,
    ) -> HashSet<String> {
        let mut result = HashSet::new();
        let document_res = self.lang_server.document_open(document_name);
        if document_res.is_ok() {
            let document = document_res.unwrap();

            let doc_symbol = self.lang_server.document_symbol(&document).unwrap();

            match doc_symbol {
                Some(DocumentSymbolResponse::Flat(_)) => {
                    println!("unsupported symbols found");
                }
                Some(DocumentSymbolResponse::Nested(doc_symbols)) => {
                    for symbol in doc_symbols {
                        if symbol.kind == SymbolKind::FUNCTION {
                            let func_name = symbol.name;
                            //println!("{}", func_name);
                            if func_filter.is_match(func_name.as_str()) {
                                result.insert(func_name.to_string());
                            }
                        }
                    }
                }
                None => {
                    println!("no symbols found");
                }
            }
        }
        result
    }
}
