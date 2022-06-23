use crate::lang_server::LanguageServer;
use crate::lang_server;
use lsp_types::{DocumentSymbolResponse, SymbolKind};
use regex::Regex;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::fs::File;
use std::hash::{Hash, Hasher};
use std::io::prelude::*;
use log::{Level, log};
use serde_json::Value;
use crate::analyzer::FilterName;

pub trait LSPServer {
    fn restart(&mut self,);
    fn find_func_name(
        &mut self,
        filter: Vec<HashMap<FilterName, Regex>>,
    ) -> HashSet<FunctionNode>;
    fn find_link(&mut self, parent_name: HashSet<String>, child_name: HashSet<String>, document_name_parent: &str, document_name_child: &str) -> HashSet<(String, String)>;
    fn close(&mut self);
}

pub struct ClangdServer {
    pub lang_server: Box<dyn LanguageServer>,
    pub project_path: String,
    pub index_map: HashMap<String, Vec<String>>,
    use_call_hierarchy_outgoing: bool,
    clangd_path: String,
}

pub struct FunctionNode {
    pub function_name: HashSet<String>,
    pub document: String,
    pub match_strategy: Box<dyn MatchFunctionEdge>
}

impl Hash for FunctionNode {
    fn hash<H: Hasher>(&self, state: &mut H) {
        for function in &self.function_name {
            function.hash(state);
        }
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
                    document: self.document.clone(),
                    match_strategy: Box::new(strategy),
                }

            }
            "ParentChildEdge" => {
                let strategy = ParentChildNode { function_name: self.function_name.clone(), document: self.document.clone() };
                FunctionNode{
                    function_name: self.function_name.clone(),
                    document: self.document.clone(),
                    match_strategy: Box::new(strategy),
                }
            }
            _ => {unimplemented!()}
        }
    }
}

pub trait MatchFunctionEdge {
    fn do_match(&mut self, match_target: FunctionNode, lsp_server: &mut Box<dyn LSPServer>) -> HashSet<(String, String)>;
    fn get_implementation(&self) -> String;
}

#[derive(Eq, PartialEq, Debug, Clone)]
pub struct ForcedNode {
    pub function_name: HashSet<String>,
    pub document: String,
}

pub struct ParentChildNode {
    pub function_name: HashSet<String>,
    pub document: String,
}

impl MatchFunctionEdge for ForcedNode {
    fn do_match(&mut self, match_target: FunctionNode, lsp_server: &mut Box<dyn LSPServer>) -> HashSet<(String, String)> {
        #[allow(dead_code)]
        if false { drop(lsp_server); unimplemented!()}
        let mut result = HashSet::new();
        for child in match_target.function_name.clone() {
            for parent in self.function_name.clone() {
                result.insert((parent.clone(), child.clone()));
            }
        }
        result
    }
    fn get_implementation(&self) -> String {
        "ForcedEdge".to_string()
    }

}

impl MatchFunctionEdge for ParentChildNode {
    fn do_match(&mut self, match_target: FunctionNode, lsp_server: &mut Box<dyn LSPServer>) -> HashSet<(String, String)> {
        lsp_server.find_link(
            self.function_name.clone(),
            match_target.function_name.clone(),
            self.document.as_str(),
            match_target.document.as_str()
        )
    }
    fn get_implementation(&self) -> String{
        "ParentChildEdge".to_string()
    }

}

impl ClangdServer {
    pub fn new(project_path: String, clangd_path: String) -> Box<dyn LSPServer> {
        let mut lsp_server = Self {
            lang_server: lang_server::LanguageServerLauncher::new()
                .server(clangd_path.to_owned())
                .project(project_path.to_owned())
                .launch()
                .expect("Failed to spawn clangd"),
            project_path,
            index_map: HashMap::new(),
            use_call_hierarchy_outgoing: true,
            clangd_path
        };
        lsp_server.get_all_files_in_project();
        let res = lsp_server.lang_server.initialize();
        if res.is_err() {
            log!(Level::Error,"LSP server didn't initialize: {:?}", res.err());
        }
        Box::new(lsp_server)
    }

    pub fn restart_server(&mut self){
        let shutdown_res = self.lang_server.exit();
        if shutdown_res.is_err() {
            log!(Level::Error, "{:?}", shutdown_res.err());
        }
        let new_lsp = lang_server::LanguageServerLauncher::new()
            .server(self.clangd_path.to_owned())
            .project(self.project_path.to_owned())
            .launch()
            .expect("Failed to spawn clangd");
        self.lang_server = new_lsp;

        let init_res = self.lang_server.initialize();
        if init_res.is_err() {
            eprintln!("{:?}", init_res.err());
        }
    }

    pub fn get_all_files_in_project(&mut self) -> Vec<String> {
        let files: Vec<String>;
        let path_to_index = self.project_path.clone() + "/.cache/clangd/index";
        let index_dir  = fs::read_dir(path_to_index.clone());

        if index_dir.is_ok() {
            let mut index_file_names: Vec<String> = vec![];
            for file in index_dir.unwrap() {
                let mut file_str = file.as_ref().unwrap().path().to_str().unwrap().to_owned();
                file_str = file_str.replace(&(path_to_index.clone() + "/"), "");
                file_str = file_str[..file_str.find(".").unwrap()].to_owned();
                index_file_names.push(file_str);
            }
            files = self.get_files_in_dir(self.project_path.clone(), self.project_path.clone(), Some(index_file_names.clone()));

            self.index_map = self.check_index_file(files.clone());
        } else {
            files = self.get_files_in_dir(self.project_path.clone(), self.project_path.clone(), None);

        }
        files
    }

    fn check_index_file(&mut self, files: Vec<String>) -> HashMap<String, Vec<String>> {
        let mut index_map : HashMap<String, Vec<String>> = HashMap::new();

        let path = self.project_path.clone() + "/.cache/index.json";
        let mut needs_indexing = false;
        let file =  File::open(&path);
        if file.is_err() {
            needs_indexing = true;
        } else {
            let mut s = String::new();
            match file.unwrap().read_to_string(&mut s) {
                Err(why) => panic!("could not read: {}", why),
                Ok(_) => {}
            }

            let json = serde_json::from_str::<Value>(s.as_str());
            if json.is_err() {
                needs_indexing = true;
            } else {
                let json_map = json.unwrap().as_object().unwrap().to_owned();
                if json_map.len() == files.len() {
                    for file in json_map{
                        if files.contains(&file.0) {
                            let mut functions: Vec<String> = vec![];
                            let symbols = file.1.as_array().unwrap().to_owned();
                            for symbol in symbols {
                                functions.push(symbol.to_string().replace("\"", ""));
                            }
                            index_map.insert(file.0, functions.clone());
                        } else {
                            needs_indexing = true;
                        }
                    }
                } else {
                    needs_indexing = true;
                }
            }

        }
        if needs_indexing {
            let mut i = 0;
            let mut i_total = 0;
            eprintln!("start indexing, there should be a message displaying the progress every coupe of seconds, please restart the program if the messages stop unexpectedly");
            for file in files.clone() {
                i += 1; i_total += 1;
                let mut functions:Vec<String> = vec![];

                if i >= 10 {
                    //break;
                    i = 0;
                    eprintln!("indexing project, please wait ({}/{})", i_total, files.clone().len());
                    self.restart_server();
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
                                        let mut func_name = symbol.name;
                                        while func_name.starts_with('_'){
                                            func_name = func_name.strip_prefix(&"_".to_string()).unwrap().to_string();
                                        }

                                        functions.push(func_name);
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

            let new_json = serde_json::to_string(&index_map).unwrap();
            let mut file_ref = File::create(path).expect("create failed");
            file_ref.write_all(new_json.as_bytes()).expect("write failed");
        }
        index_map
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
    fn restart(&mut self) {
        self.restart_server();
    }

    fn find_func_name(
        &mut self,
        filter: Vec<HashMap<FilterName, Regex>>,
    ) -> HashSet<FunctionNode> {
        let mut func_nodes:HashSet<FunctionNode> = HashSet::new();

        for f in filter {

            let mut forced = false;
            let mut only_ident = false;
            let mut ident = String::new();
            let mut file_filter = Regex::new(".").unwrap();
            let mut function_filter = Regex::new(".").unwrap();


            if f.contains_key(&FilterName::Forced) {
                forced = true;
            }

            if f.contains_key(&FilterName::FunctionNameFromIdent) {
                ident = f.get(&FilterName::FunctionNameFromIdent).unwrap().to_string();
                only_ident = true;
            }

            if f.contains_key(&FilterName::File) {
                let regex = f.get(&FilterName::File).unwrap();
                file_filter = regex.to_owned();
            }

            if f.contains_key(&FilterName::Function) {
                let regex = f.get(&FilterName::Function).unwrap();
                function_filter = regex.to_owned();
            }

            for document in self.index_map.clone() {
                let file = document.0;
                if file_filter.is_match(file.as_str()) {
                    let mut function_names: HashSet<String> = HashSet::new();
                    for function in document.1.clone() {
                        let mut found = false;
                        if only_ident == true {
                            if ident == function {
                                found = true
                            }
                        } else {
                            if function_filter.is_match(function.as_str()) {
                                found = true;
                            }
                        }
                        if found {
                            function_names.insert(function.clone());
                        }
                    }
                    if function_names.len() > 0 {
                        if forced {
                            let node = ForcedNode {
                                function_name: function_names.clone(),
                                document: file.clone(),
                            };
                            func_nodes.insert(FunctionNode { function_name: function_names.clone(), document: file.clone(), match_strategy: Box::new(node) });
                        } else {
                            let node = ParentChildNode {
                                function_name: function_names.clone(),
                                document: file.clone()
                            };
                            func_nodes.insert(FunctionNode { function_name: function_names.clone(), document: file.clone(), match_strategy: Box::new(node) });
                        }
                    }
                }
            }

            if func_nodes.len() == 0 && function_filter.as_str() != "." {
                let mut hash_set = HashSet::new();
                hash_set.insert(function_filter.as_str().to_string());
                let node = ParentChildNode {
                    function_name: hash_set,
                    document: "not found".to_string()
                };
                func_nodes.insert(FunctionNode{function_name: node.function_name.clone(), document: "not found".to_string(), match_strategy: Box::new(node)});

            }
        }

        func_nodes
    }

    fn find_link(&mut self, parent_name: HashSet<String>, child_name: HashSet<String>, document_name_parent: &str, document_name_child: &str) -> HashSet<(String, String)> {
        //println!("{:?} -> {:?} @{}", parent_name, child_name, document_name_parent);
        let mut connections : HashSet<(String, String)> = HashSet::new();

        let mut need_lsp = true;



        if parent_name.len() > child_name.len() {
            for child in child_name{

                    let path = self.project_path.clone() + "/" + document_name_child;

                    let file = File::open(&path);
                    if file.is_ok() {
                        let mut s = String::new();
                        match file.unwrap().read_to_string(&mut s) {
                            Err(why) => panic!("could not read: {}", why),
                            Ok(_) => {}
                        }
                        need_lsp = s.contains(&child);
                    }
                    if need_lsp {
                        let mut parents: HashSet<String> = HashSet::new();

                        let document = self.lang_server.document_open(document_name_child).unwrap();

                        let doc_symbol = self.lang_server.document_symbol(&document).unwrap();

                        match doc_symbol {
                            Some(DocumentSymbolResponse::Flat(token)) => {
                                log!(Level::Warn ,"unsupported symbols found {:?}", token);
                            }
                            Some(DocumentSymbolResponse::Nested(doc_symbols)) => {
                                for symbol in doc_symbols {
                                    if symbol.kind == SymbolKind::FUNCTION {
                                        let mut func_name = symbol.name;
                                        while func_name.starts_with('_') {
                                            func_name = func_name.strip_prefix(&"_".to_string()).unwrap().to_string();
                                        }
                                        if func_name == child {
                                            let prep_call_hierarchy = self
                                                .lang_server
                                                .call_hierarchy_item(&document, symbol.range.start);
                                            let call_hierarchy_array = prep_call_hierarchy.unwrap().unwrap();
                                            if call_hierarchy_array.len() == 0 {
                                                let mut i = 0;
                                                self.restart_server();
                                                let path2 = self.project_path.clone() + "/" + document_name_parent;
                                                let file2 = File::open(&path2);
                                                if file2.is_ok() {
                                                    let mut s2 = String::new();
                                                    match file2.unwrap().read_to_string(&mut s2) {
                                                        Err(why) => panic!("could not read: {}", why),
                                                        Ok(_) => {}
                                                    }

                                                    let filter_str = (child.clone() + "(").clone();
                                                    if s2.contains(&filter_str) {
                                                        i += 1;
                                                        if i >= 5 {
                                                            i = 0;
                                                            self.restart_server();
                                                        }
                                                        let search_document_resp = self.lang_server.document_open(&document_name_parent).unwrap();
                                                        let search_doc_symbol = self.lang_server.document_symbol(&search_document_resp).unwrap();
                                                        let search_nested_symbol = search_doc_symbol.unwrap();
                                                        match search_nested_symbol {
                                                            DocumentSymbolResponse::Flat(_) => {
                                                                log!(Level::Warn ,"unsupported symbols found");
                                                            }
                                                            DocumentSymbolResponse::Nested(search_symbols) => {
                                                                let doc_lines: Vec<&str> = s2.split("\n").collect();
                                                                let mut j: u32 = 0;
                                                                while (j as usize) < doc_lines.len() {
                                                                    if doc_lines[(j as usize)].contains(&filter_str) {
                                                                        for search_symbol in search_symbols.clone() {
                                                                            if search_symbol.range.start.line < j && j <= search_symbol.range.end.line {
                                                                                parents.insert(search_symbol.name.clone());
                                                                            }
                                                                        }
                                                                    }
                                                                    j += 1;
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                            for call_hierarchy_item in call_hierarchy_array {
                                                let outgoing_calls = self
                                                    .lang_server
                                                    .call_hierarchy_item_incoming(call_hierarchy_item.clone());
                                                if outgoing_calls.is_ok() {
                                                    for outgoing_call in outgoing_calls.unwrap().unwrap() {
                                                        if outgoing_call.from.kind == SymbolKind::FUNCTION {
                                                            parents.insert(outgoing_call.from.name.clone());
                                                        }
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
                        for parent in parents {
                            if parent_name.contains(&parent) {
                                connections.insert((parent.clone(), child.clone()));
                            }

                    }
                }
            }
        } else {
            for parent in parent_name {
                let path = self.project_path.clone() + "/" + document_name_parent;

                let file = File::open(&path);
                if file.is_ok() {
                    let mut s = String::new();
                    match file.unwrap().read_to_string(&mut s) {
                        Err(why) => panic!("could not read: {}", why),
                        Ok(_) => {}
                    }
                    need_lsp = s.contains(&parent);
                }

                if need_lsp {
                    let mut children: HashSet<String> = HashSet::new();

                    let document = self.lang_server.document_open(document_name_parent).unwrap();

                    let doc_symbol = self.lang_server.document_symbol(&document).unwrap();

                    match doc_symbol {
                        Some(DocumentSymbolResponse::Flat(token)) => {
                            log!(Level::Warn ,"unsupported symbols found {:?}", token);
                        }
                        Some(DocumentSymbolResponse::Nested(doc_symbols)) => {
                            for symbol in doc_symbols {
                                if symbol.kind == SymbolKind::FUNCTION {
                                    let mut func_name = symbol.name;
                                    while func_name.starts_with('_') {
                                        func_name = func_name.strip_prefix(&"_".to_string()).unwrap().to_string();
                                    }
                                    if func_name == parent {
                                        let mut unsuccessful_response;
                                        if self.use_call_hierarchy_outgoing {
                                            let prep_call_hierarchy = self
                                                .lang_server
                                                .call_hierarchy_item(&document, symbol.range.start);
                                            let call_hierarchy_array = prep_call_hierarchy.unwrap().unwrap();
                                            unsuccessful_response = true;
                                            for call_hierarchy_item in call_hierarchy_array {
                                                let outgoing_calls = self
                                                    .lang_server
                                                    .call_hierarchy_item_outgoing(call_hierarchy_item.clone());
                                                if outgoing_calls.is_ok() {
                                                    unsuccessful_response = true;
                                                    for outgoing_call in outgoing_calls.unwrap().unwrap() {
                                                        unsuccessful_response = false;
                                                        if outgoing_call.to.kind == SymbolKind::FUNCTION {
                                                                children.insert(outgoing_call.to.name.clone());
                                                        }
                                                    }
                                                } else {
                                                    unsuccessful_response = true;
                                                    self.use_call_hierarchy_outgoing = false;
                                                }
                                            }
                                        } else {
                                            unsuccessful_response = true;
                                        }
                                        if unsuccessful_response {
                                            let doc_text = document.text.clone();
                                            let doc_lines: Vec<&str> = doc_text.split("\n").collect();
                                            let start: usize = (symbol.range.start.line + 1) as usize;
                                            let end: usize = symbol.range.end.line as usize;
                                            if start < end {
                                                let function_data = doc_lines[start..end].concat();
                                                for child in child_name.clone() {
                                                    let search_name = child.clone() + "(";
                                                    if function_data.contains(&search_name) {
                                                        children.insert(child);
                                                    }
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

                    for child in children{
                        if child_name.contains(&child) {
                            connections.insert((parent.clone(), child.clone()));
                        }
                    }
                }


            }
        }
        connections
    }

    fn close(&mut self){
        log!(Level::Info, "{:?}", self.lang_server.shutdown());
        log!(Level::Info, "{:?}", self.lang_server.exit());
    }
}
