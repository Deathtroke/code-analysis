use crate::lang_server::LanguageServer;
use crate::{lang_server, parser};
use lsp_types::{DocumentSymbolResponse, SymbolKind};
use regex::Regex;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::fs::File;
use std::hash::{Hash, Hasher};
use std::io::prelude::*;
use log::{Level, log};
use crate::parser::FilterName;

pub trait LSPServer {
    fn search_connection_filter(
        &mut self,
        parent_name: String,
        child_name: String,
    ) -> HashSet<(String, String)>;
    fn find_func_name(
        &mut self,
        filter: Vec<HashMap<FilterName, Regex>>,
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
    fn find_functions_in_doc(&mut self, func_filter: Regex, ident: Option<String>, document_name: &str)
        -> HashSet<String>;
    fn close(&mut self);
}

pub struct ClangdServer {
    pub lang_server: Box<dyn LanguageServer>,
    pub files_in_project: Vec<String>,
    pub project_path: String,
    pub index_map: HashMap<String, Vec<String>>,
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
                let strategy = ForcedNode { function_name: self.function_name.clone(), document: self.document.clone() };
                FunctionNode{
                    function_name: self.function_name.clone(),
                    document: self.function_name.clone(),
                    match_strategy: Box::new(strategy),
                }

            }
            "ParentChildEdge" => {
                let strategy = ParentChildNode { function_name: self.function_name.clone(), document: self.document.clone() };
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
pub struct ForcedNode {
    pub function_name: String,
    pub document: String,
}

pub struct ParentChildNode {
    pub function_name: String,
    pub document: String,
}

impl MatchFunctionEdge for ForcedNode {
    fn do_match(&mut self, match_target: FunctionNode, lsp_server: &mut Box<dyn LSPServer>) -> bool {
        #[allow(dead_code)]
        if false { drop(match_target); drop(lsp_server); unimplemented!()}
        true
    }
    fn get_implementation(&self) -> String {
        "ForcedEdge".to_string()
    }

}

impl MatchFunctionEdge for ParentChildNode {
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
                .server("/usr/bin/clangd-14".to_owned())
                .project(project_path.to_owned())
                .launch()
                .expect("Failed to spawn clangd"),
            files_in_project: vec![],
            project_path,
            index_map: HashMap::new(),
        };
        lsp_server.files_in_project = lsp_server.get_all_files_in_project();
        let res = lsp_server.lang_server.initialize();
        if res.is_err() {
            log!(Level::Error,"LSP server didn't initialize: {:?}", res.err());
        }
        Box::new(lsp_server)
    }

    pub fn get_all_files_in_project(&mut self) -> Vec<String> {
        let mut files: Vec<String> = Vec::new();
        let path_to_index = self.project_path.clone() + "/.cache/clangd/index";
        let index_dir  = fs::read_dir(path_to_index.clone());
        let mut index_map : HashMap<String, Vec<String>> = HashMap::new();


        if index_dir.is_ok() {
            let mut index_file_names: Vec<String> = vec![];
            for file in index_dir.unwrap() {
                let mut file_str = file.as_ref().unwrap().path().to_str().unwrap().to_owned();
                file_str = file_str.replace(&(path_to_index.clone() + "/"), "");
                file_str = file_str[..file_str.find(".").unwrap()].to_owned();
                //println!("{}", file.as_ref().unwrap().path().to_str().unwrap().to_owned());
                index_file_names.push(file_str);
            }
            files = self.get_files_in_dir(self.project_path.clone(), self.project_path.clone(), Some(index_file_names.clone()));

            let mut i = 0;
            for file in files.clone() {
                i+=1;
                let mut functions:Vec<String> = vec![];

                //println!("{} {}", i, file);

                if i > 30 {
                    //break;
                    i = 0;
                    println!("indexing project, please wait");
                    //println!("{:?}", self.lang_server.shutdown());
                    let shutdown_res = self.lang_server.exit();
                    if shutdown_res.is_err() {
                        println!("{:?}", shutdown_res.err());
                    }
                    let new_lsp = lang_server::LanguageServerLauncher::new()
                        .server("/usr/bin/clangd-14".to_owned())
                        .project(self.project_path.to_owned())
                        .launch()
                        .expect("Failed to spawn clangd");
                    self.lang_server = new_lsp;

                    let init_res = self.lang_server.initialize();
                    if init_res.is_err() {
                        println!("{:?}", init_res.err());
                    }
                }
                let document_res = self.lang_server.document_open(file.as_str());
                if document_res.is_ok() {
                    let document = document_res.unwrap();
                    let doc_symbol = self.lang_server.document_symbol(&document);
                    if doc_symbol.is_ok() {
                        match doc_symbol.unwrap() {
                            Some(DocumentSymbolResponse::Flat(_)) => {
                                log!(Level::Warn ,"unsupported symbols found");
                            }
                            Some(DocumentSymbolResponse::Nested(doc_symbols)) => {
                                for symbol in doc_symbols {
                                    if symbol.kind == SymbolKind::FUNCTION {
                                        functions.push(symbol.name);
                                    }
                                }
                            }
                            None => {
                                log!(Level::Warn, "no symbols found");
                            }
                        }
                    }
                }
                index_map.insert(file, functions);
            }

        } else {
            files = self.get_files_in_dir(self.project_path.clone(), self.project_path.clone(), None);

        }
        self.index_map = index_map;
        files
    }

    fn get_files_in_dir(&self, dir: String, project_path: String, index_file_name: Option<Vec<String>>) -> Vec<String> {
        let mut files: Vec<String> = Vec::new();

        let paths = fs::read_dir(dir.clone()).unwrap();

        for path in paths {
            let path_str = path.as_ref().unwrap().path().to_str().unwrap().to_string();
            if path.as_ref().unwrap().metadata().unwrap().is_dir() {
                let mut subfolder = self.get_files_in_dir(path_str, project_path.clone(), index_file_name.clone());
                files.append(&mut subfolder);
            } else {
                if index_file_name.clone().is_some() {
                    let mut name = path_str.replace(&(project_path.clone().as_str().to_owned() + "/"), "");
                    while name.find("/").is_some() {
                        name = name[(name.find("/").unwrap()+1)..].to_owned();
                    }
                    if name.find(".").is_some() {
                        name = name[..name.find(".").unwrap()].to_owned();

                        if index_file_name.clone().unwrap().contains(&name){
                            if path_str.ends_with(".cpp") || path_str.ends_with(".c") {
                                files.push(path_str.replace(&(project_path.clone().as_str().to_owned() + "/"), ""));
                            }
                        }
                    }

                } else {
                    if path_str.ends_with(".cpp") || path_str.ends_with(".c") {
                        files.push(path_str.replace(&(project_path.clone().as_str().to_owned() + "/"), ""));
                    }
                }
            }
        }
        files
    }
}

impl LSPServer for ClangdServer {
    fn search_connection_filter(
        &mut self,
        parent_name: String,
        child_name: String,
    ) -> HashSet<(String, String)> {
        let mut connections: HashSet<(String, String)> = HashSet::new();

        if parent_name.as_str() == "" {
            for file_path in self.files_in_project.clone() {
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
                if s.contains(&child_name.clone()) {
                    new_children = self.search_parent_single_document_filter(
                        Regex::new(child_name.as_str()).unwrap(),
                        HashMap::new(),
                        file_path.as_str(),
                    );
                }
                for child in new_children {
                    connections.insert(child);
                }
            }
        } else {
            for file_path in self.files_in_project.clone() {
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
                if s.contains(&parent_name.clone()) {
                    new_children = self.search_child_single_document_filter(
                        Regex::new(parent_name.as_str()).unwrap(),
                        HashMap::new(),
                        file_path.as_str(),
                    );
                }
                for child in new_children {
                    connections.insert(child);
                }
            }
        }
        connections
    }

    fn find_func_name(
        &mut self,
        filter: Vec<HashMap<FilterName, Regex>>,
    ) -> HashSet<FunctionNode> {
        let mut func_names: HashSet<FunctionNode> = HashSet::new();
        for f in filter {
            let mut forced = false;
            if f.contains_key(&FilterName::Forced) {
                forced = true;
            }
            let mut only_ident = false;
            let mut ident = String::new();
            if f.contains_key(&FilterName::FunctionNameFromIdent) {
                ident = f.get(&FilterName::FunctionNameFromIdent).unwrap().to_string();
                only_ident = true;
            }

            let mut file_filter = Regex::new(".").unwrap();
            if f.contains_key(&FilterName::File) {
                let regex = f.get(&FilterName::File).unwrap();
                file_filter = regex.to_owned();
            }

            let mut function_filter = Regex::new(".").unwrap();
            if f.contains_key(&FilterName::Function) {
                let regex = f.get(&FilterName::Function).unwrap();
                function_filter = regex.to_owned();
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

                    let need_lsp = (function_filter.is_match(s.as_str()) && !only_ident) || (s.contains(&ident.clone()) && only_ident);

                    if need_lsp {
                        let ident_opt :Option<String> = if only_ident {
                             Some(ident.clone())
                        } else {
                            None
                        };

                        let names =
                            self.find_functions_in_doc(function_filter.clone(), ident_opt, file_path.as_str());
                        for name in names {
                            if forced {
                                let node = ForcedNode {
                                    function_name: name.clone(),
                                    document: file_path.clone()
                                };
                                func_names.insert( FunctionNode{function_name: name.clone(), document: file_path.clone(), match_strategy: Box::new(node)});
                            } else {
                                let node = ParentChildNode {
                                    function_name: name.clone(),
                                    document: file_path.clone()
                                };
                                func_names.insert( FunctionNode{function_name: name.clone(), document: file_path.clone(), match_strategy: Box::new(node)});

                            }
                        }
                    }
                }
            }
            if func_names.len() == 0 && function_filter.as_str() != "." {
                let node = ParentChildNode {
                    function_name: function_filter.as_str().to_string(),
                    document: "not found".to_string()
                };
                func_names.insert( FunctionNode{function_name: node.function_name.clone(), document: node.document.clone(), match_strategy: Box::new(node)});

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
            Some(DocumentSymbolResponse::Flat(token)) => {
                log!(Level::Warn ,"unsupported symbols found {:?}", token);
            }
            Some(DocumentSymbolResponse::Nested(doc_symbols)) => {
                for symbol in doc_symbols {
                    if symbol.kind == SymbolKind::FUNCTION {
                        let func_name = symbol.name;
                        if func_filter.is_match(func_name.as_str()) {
                            let prep_call_hierarchy = self
                                .lang_server
                                .call_hierarchy_item(&document, symbol.range.start);
                            let call_hierarchy_array = prep_call_hierarchy.unwrap().unwrap();
                            for call_hierarchy_item in call_hierarchy_array{
                                let outgoing_calls = self
                                    .lang_server
                                    .call_hierarchy_item_outgoing(call_hierarchy_item.clone());
                                if outgoing_calls.is_ok() {
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
                                } else {
                                    println!("{:?}", outgoing_calls.as_ref());
                                    println!("{:?}", self.index_map);
                                }
                            }
                        }
                    }
                }
            }
            None => {
                log!(Level::Warn, "no symbols found");
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
                log!(Level::Warn ,"unsupported symbols found");
            }
            Some(DocumentSymbolResponse::Nested(doc_symbols)) => {
                for symbol in doc_symbols {
                    if symbol.kind == SymbolKind::FUNCTION {
                        let func_name = symbol.name;
                        if func_filter.is_match(func_name.as_str()) {
                            let prep_call_hierarchy = self
                                .lang_server
                                .call_hierarchy_item(&document, symbol.range.start);
                            let call_hierarchy_array = prep_call_hierarchy.unwrap().unwrap();
                            for call_hierarchy_item in call_hierarchy_array {
                                let incoming_calls = self
                                    .lang_server
                                    .call_hierarchy_item_incoming(call_hierarchy_item.clone());
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
                log!(Level::Warn, "no symbols found");
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
                    log!(Level::Warn ,"unsupported symbols found");
                }
                Some(DocumentSymbolResponse::Nested(doc_symbols)) => {
                    for symbol in doc_symbols {
                        if symbol.kind == SymbolKind::FUNCTION {
                            let func_name = symbol.name;
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
                    log!(Level::Warn, "no symbols found");
                }
            }
        }
        false
    }

    fn find_functions_in_doc(
        &mut self,
        func_filter: Regex,
        ident: Option<String>,
        document_name: &str,
    ) -> HashSet<String> {
        let mut result = HashSet::new();
        let document_res = self.lang_server.document_open(document_name);
        if document_res.is_ok() {
            let document = document_res.unwrap();

            let doc_symbol = self.lang_server.document_symbol(&document).unwrap();

            match doc_symbol {
                Some(DocumentSymbolResponse::Flat(_)) => {
                    log!(Level::Warn ,"unsupported symbols found");
                }
                Some(DocumentSymbolResponse::Nested(doc_symbols)) => {
                    for symbol in doc_symbols {
                        if symbol.kind == SymbolKind::FUNCTION {
                            let func_name = symbol.name;
                            if ident.clone().is_some() {
                                if func_name == ident.clone().unwrap() {
                                    result.insert(func_name.to_string());

                                }
                            } else {
                                if func_filter.is_match(func_name.as_str()) {
                                    result.insert(func_name.to_string());

                                }
                            }
                        }
                    }
                }
                None => {
                    log!(Level::Warn, "no symbols found");
                }
            }
        }
        result
    }

    fn close(&mut self){
        self.lang_server.shutdown();
        self.lang_server.exit();
    }
}
