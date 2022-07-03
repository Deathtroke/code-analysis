use crate::lang_server::LanguageServer;
use crate::lang_server;
use lsp_types::{DocumentSymbolResponse, Range, SymbolKind};
use regex::Regex;
use std::collections::{HashMap, HashSet};
use std::fmt::{Debug, Formatter};
use std::{fmt, fs};
use std::fs::File;
use std::hash::{Hash, Hasher};
use std::io::prelude::*;
use chrono::Utc;
use log::{Level, log};
use serde_json::Value;
use crate::analyzer::FilterName;

pub trait LSPServer {
    fn restart(&mut self,);
    fn find_func_name(
        &mut self,
        filter: Vec<HashMap<FilterName, Regex>>,
    ) -> HashSet<FunctionNode>;
    fn find_link(&mut self, parent_name: HashSet<String>, child_name: HashSet<String>) -> HashSet<(String, String)>;
    fn close(&mut self);
}

pub struct ClangdServer {
    pub lang_server: Box<dyn LanguageServer>,
    pub project_path: String,
    pub index_map: HashMap<String, Vec<String>>,
    function_index: HashMap<String, Vec<String>>,
    inv_function_index: HashMap<String, Vec<String>>,
    use_call_hierarchy_outgoing: bool,
    clangd_path: String,
    benchmark: bool,
}

pub struct FunctionNode {
    pub function_name: HashSet<String>,
    pub match_strategy: Box<dyn MatchFunctionEdge>
}

impl Hash for FunctionNode {
    fn hash<H: Hasher>(&self, state: &mut H) {
        for function in &self.function_name {
            function.hash(state);
        }
    }
}

impl PartialEq for FunctionNode {
    fn eq(&self, other: &Self) -> bool {
        self.function_name == other.function_name
    }
}

impl Eq for FunctionNode {}

impl Clone for FunctionNode {
    fn clone(&self) -> Self {
        match self.match_strategy.get_implementation().as_str() {
            "ForcedEdge" => {
                let strategy = ForcedNode { function_name: self.function_name.clone() };
                FunctionNode{
                    function_name: self.function_name.clone(),
                    match_strategy: Box::new(strategy),
                }

            }
            "ParentChildEdge" => {
                let strategy = ParentChildNode { function_name: self.function_name.clone() };
                FunctionNode{
                    function_name: self.function_name.clone(),
                    match_strategy: Box::new(strategy),
                }
            }
            _ => {unimplemented!()}
        }
    }
}

impl fmt::Debug for FunctionNode{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("FunctionNode")
            .field("functions", &self.function_name)
            .field("match srtategy", &self.match_strategy.get_implementation())
            .finish()
    }
}

pub trait MatchFunctionEdge {
    fn do_match(&mut self, match_target: FunctionNode, lsp_server: &mut Box<dyn LSPServer>) -> HashSet<(String, String)>;
    fn get_implementation(&self) -> String;
}

#[derive(Eq, PartialEq, Debug, Clone)]
pub struct ForcedNode {
    pub function_name: HashSet<String>,
}

pub struct ParentChildNode {
    pub function_name: HashSet<String>,
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
            match_target.function_name.clone(),
            self.function_name.clone()
        )
    }
    fn get_implementation(&self) -> String{
        "ParentChildEdge".to_string()
    }

}

impl ClangdServer {
    pub fn new(project_path: String, clangd_path: String, benchmark: (chrono::NaiveTime, bool)) -> Box<dyn LSPServer> {
        let mut lsp_server = Self {
            lang_server: lang_server::LanguageServerLauncher::new()
                .server(clangd_path.to_owned())
                .project(project_path.to_owned())
                .launch()
                .expect("Failed to spawn clangd"),
            project_path,
            index_map: HashMap::new(),
            function_index: Default::default(),
            inv_function_index: Default::default(),
            use_call_hierarchy_outgoing: true,
            clangd_path,
            benchmark: benchmark.1
        };
        let res = lsp_server.lang_server.initialize();
        if res.is_err() {
            log!(Level::Error,"LSP server didn't initialize: {:?}", res.err());
        }
        lsp_server.get_all_files_in_project();
        if benchmark.1 {
            let now = Utc::now().time();
            let diff = now - benchmark.0;
            eprintln!("Time till index is finished: {} ms", diff.num_milliseconds());
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
        let mut range_index: HashMap<String, Vec<Range>> = HashMap::new();

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

        let file2 =  File::open(self.project_path.clone() + "/.cache/called.json");
        if file2.is_err() {
            needs_indexing = true;
        } else {
            let mut s = String::new();
            match file2.unwrap().read_to_string(&mut s) {
                Err(why) => panic!("could not read: {}", why),
                Ok(_) => {}
            }

            let json2 = serde_json::from_str::<Value>(s.as_str());
            if json2.is_err() {
                needs_indexing = true;
            } else {
                let json_map = json2.unwrap().as_object().unwrap().to_owned();
                if json_map.len() > 1 {
                    for file in json_map{
                        let mut functions: Vec<String> = vec![];
                        let symbols = file.1.as_array().unwrap().to_owned();
                        for symbol in symbols {
                            functions.push(symbol.to_string().replace("\"", ""));
                        }
                        self.function_index.insert(file.0, functions.clone());
                    }
                } else {
                    needs_indexing = true;
                }
            }

        }
        let file3 =  File::open(self.project_path.clone() + "/.cache/caller.json");
        if file3.is_err() {
            needs_indexing = true;
        } else {
            let mut s = String::new();
            match file3.unwrap().read_to_string(&mut s) {
                Err(why) => panic!("could not read: {}", why),
                Ok(_) => {}
            }

            let json3 = serde_json::from_str::<Value>(s.as_str());
            if json3.is_err() {
                needs_indexing = true;
            } else {
                let json_map = json3.unwrap().as_object().unwrap().to_owned();
                if json_map.len() > 1 {
                    for file in json_map{
                        let mut functions: Vec<String> = vec![];
                        let symbols = file.1.as_array().unwrap().to_owned();
                        for symbol in symbols {
                            functions.push(symbol.to_string().replace("\"", ""));
                        }
                        self.inv_function_index.insert(file.0, functions.clone());
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
                let mut ranges: Vec<Range> = vec![];

                if i >= 10 {
                    //break;
                    i = 0;
                    eprintln!("indexing project, please wait ({}/{})", i_total, files.clone().len());
                    self.restart_server();
                }
                let start = Utc::now().time();
                let document_res = self.lang_server.document_open(file.as_str());
                if self.benchmark {
                    let finish = Utc::now().time();
                    let diff = finish-start;
                    eprintln!("lsp: document_open: {:?}", diff.num_milliseconds());
                }
                if document_res.is_ok() {
                    let document = document_res.unwrap();

                    let start = Utc::now().time();
                    let doc_symbol = self.lang_server.document_symbol(&document);
                    if self.benchmark {
                        let finish = Utc::now().time();
                        let diff = finish-start;
                        eprintln!("lsp: document_symbol: {:?}", diff.num_milliseconds());
                    }

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
                                        ranges.push(symbol.range.clone());
                                    }
                                }
                            }
                            None => {
                                log!(Level::Warn, "no symbols found");
                            }
                        }
                    }
                }
                index_map.insert(file.clone(), functions);
                range_index.insert(file.clone(), ranges);
            }
            if self.benchmark {
                let document_res = self.lang_server.document_open("/criu/fsnotify.c");
                if document_res.is_ok() {
                    let document = document_res.unwrap();
                    let doc_symbol = self.lang_server.document_symbol(&document);
                    if doc_symbol.is_ok() {
                        match doc_symbol.unwrap() {
                            Some(DocumentSymbolResponse::Flat(_)) => {}
                            Some(DocumentSymbolResponse::Nested(doc_symbols)) => {
                                let symbol = doc_symbols[6].to_owned();
                                println!("{:?}", symbol);
                                let start = Utc::now().time();
                                let hierarchy_item = self.lang_server.call_hierarchy_item(&document, symbol.range.start).unwrap().unwrap()[0].clone();
                                let finish = Utc::now().time();
                                let diff = finish-start;
                                eprintln!("lsp: call_hierarchy_item: {:?}", diff.num_milliseconds());
                                let start = Utc::now().time();
                                self.lang_server.call_hierarchy_item_incoming(hierarchy_item.clone());
                                let finish = Utc::now().time();
                                let diff = finish-start;
                                eprintln!("lsp: call_hierarchy_item_incomming: {:?}", diff.num_milliseconds());
                                let start = Utc::now().time();
                                self.lang_server.call_hierarchy_item_outgoing(hierarchy_item);
                                let finish = Utc::now().time();
                                let diff = finish-start;
                                eprintln!("lsp: call_hierarchy_item_outgoing: {:?}", diff.num_milliseconds());

                            }
                            None => {}
                        }
                    }
                }
            }

            let new_json = serde_json::to_string(&index_map).unwrap();
            let mut file_ref = File::create(path).expect("create failed");
            file_ref.write_all(new_json.as_bytes()).expect("write failed");

            eprintln!("Done Step 1. Now indexing all the function calls. Please wait a little further");
            let mut i = 0;
            for document in index_map.clone() {
                i += 1;
                if i % 25 == 0 {
                    eprintln!("indexing functions, please wait ({}%)", i*100/index_map.len());

                }
                let ranges = range_index.get(document.0.clone().as_str()).unwrap().to_owned();
                let functions = document.1.clone();
                let max = ranges.len();
                for i in 0..max {
                    let name = functions[i].clone();
                    let range = ranges[i].clone();
                    let mut called_functions = Vec::new();

                    let doc_path = self.project_path.clone() + "/" + document.0.clone().as_str();
                    let file =  File::open(&doc_path);
                    if file.is_err() {
                        needs_indexing = true;
                    } else {
                        let mut s = String::new();
                        match file.unwrap().read_to_string(&mut s) {
                            Err(why) => panic!("could not read: {}", why),
                            Ok(_) => {}
                        }
                        let doc_lines: Vec<&str> = s.split("\n").collect();
                        let start: usize = (range.start.line + 1) as usize;
                        let end: usize = range.end.line as usize;
                        if start < end {
                            let function_data = doc_lines[start..end].concat();
                            for doc_2 in index_map.clone() {
                                let func_names = doc_2.1;
                                for func_name in func_names {
                                    let search_name = func_name.clone() + "(";
                                    if function_data.contains(&search_name) {
                                        called_functions.push(func_name.clone());
                                        let mut caller_function: Vec<String> = Vec::new();
                                        if self.inv_function_index.contains_key(func_name.clone().as_str()) {
                                            caller_function = self.inv_function_index.get(func_name.clone().as_str()).unwrap().to_owned();
                                        }
                                        caller_function.push(name.clone());
                                        self.inv_function_index.insert(func_name.clone(), caller_function.clone());
                                    }
                                }
                            }
                        }
                    }
                    self.function_index.insert(name, called_functions);
                }
            }


            let new_json2 = serde_json::to_string(&self.function_index).unwrap();
            let mut file_ref2 = File::create(self.project_path.clone() + "/.cache/called.json").expect("create failed");
            file_ref2.write_all(new_json2.as_bytes()).expect("write failed");
            let new_json3 = serde_json::to_string(&self.inv_function_index).unwrap();
            let mut file_ref3 = File::create(self.project_path.clone() + "/.cache/caller.json").expect("create failed");
            file_ref3.write_all(new_json3.as_bytes()).expect("write failed");

        }
        else {
            //eprintln!("done loading index files");
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

            let mut function_names: HashSet<String> = HashSet::new();

            for document in self.index_map.clone() {
                let file = document.0;
                if file_filter.is_match(file.as_str()) {
                    for function in document.1.clone() {
                        let mut found = false;
                        if only_ident {
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
                }
            }
            if function_names.len() == 0 && only_ident {
                function_names.insert(ident);
            }
            if function_names.len() > 0 {
                if forced {
                    let node = ForcedNode {
                        function_name: function_names.clone(),
                    };
                    func_nodes.insert(FunctionNode { function_name: function_names.clone(), match_strategy: Box::new(node) });
                } else {
                    let node = ParentChildNode {
                        function_name: function_names.clone(),
                    };
                    func_nodes.insert(FunctionNode { function_name: function_names.clone(), match_strategy: Box::new(node) });
                }
            }
        }

        func_nodes
    }

    fn find_link(&mut self, parent_name: HashSet<String>, child_name: HashSet<String>) -> HashSet<(String, String)> {
        //println!("{:?} -> {:?}", parent_name, child_name);
        let mut connections : HashSet<(String, String)> = HashSet::new();


        if parent_name.len() > child_name.len() {
            for child in child_name{
                if self.inv_function_index.contains_key(child.clone().as_str()){
                    let caller_names = self.inv_function_index.get(child.clone().as_str()).unwrap().to_owned();
                    for name in caller_names {
                        if parent_name.contains(name.clone().as_str()){
                            let p_name = parent_name.get(name.clone().as_str()).unwrap().to_owned();
                            connections.insert((p_name.clone(),child.clone()));
                        }
                    }
                }
            }
        } else {
            for parent in parent_name {
                if self.function_index.contains_key(parent.clone().as_str()){
                    let called_names = self.function_index.get(parent.clone().as_str()).unwrap().to_owned();
                    for name in called_names {
                        if child_name.contains(name.clone().as_str()){
                            let c_name = child_name.get(name.clone().as_str()).unwrap().to_owned();
                            connections.insert((parent.clone(),c_name.clone()));
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
