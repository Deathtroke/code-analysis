use super::*;
use std::collections::HashSet;

#[cfg(test)]
impl LSPInterface for parser {
    fn search_parent(&mut self, search_target: String)  -> HashSet<String>{
        let parents :HashSet<String> = HashSet::from(["parent1".to_string(), "parent2".to_string()]);

        for parent in parents.clone() {
            self.graph.insert_edge(None, parent, search_target.to_string());

        }
        parents
    }

    fn search_child(&mut self, search_target: String)  -> HashSet<String>{
        let mut children :HashSet<String> = HashSet::from(["child1".to_string(), "child2".to_string()]);
        for child in children.clone() {
            self.graph.insert_edge(None, search_target.to_string(), child);
        }
        children
    }

    fn paren_child_exists(&mut self, parent: String, child: String) -> bool{
        let result = (parent == "parent1" || parent == "parent2") &&
            (child == "child1" || child == "child2") ;

        if result {
            self.graph.insert_edge(None, parent, child);

        }

        result
    }

    fn search_connection_filter(&mut self, parent_filter: HashMap<String, String>, child_filter: HashMap<String, String>) -> HashSet<(String, String)> {
        let mut result: HashSet<(String, String)> = HashSet::new();
        for parent in parent_filter.clone() {
            for child in child_filter.clone() {
                result.insert((parent.1.clone(), child.1.clone()));
            }
        }
        result
    }

    fn find_func_name(&mut self, filter: Vec<HashMap<FilterName, String>>) -> HashSet<FunctionEdge> {
        let mut result :HashSet<FunctionEdge> = HashSet::new();
        for f in filter {
            if f.contains_key(&FilterName::Function) {
                result.insert(FunctionEdge{
                    function_name: f.get(&FilterName::Function).unwrap().clone(),
                    document: "".to_string(),
                });
            }
        }
        result
    }
}

#[test]
fn test_parser_simple1() {
    let input = r#"{@func}"#;
    let mut parser = parser::new("/Users/hannes.boerner/Downloads/criu-criu-dev".to_string());
    let parser_output = HashSet::from(["parent1".to_string(), "parent2".to_string()]);
    assert_eq!(parser.parse(input), parser_output);
    let graph_output = HashSet::from([("parent1".to_string(), "func".to_string()), ("parent2".to_string(), "func".to_string())]);
    assert_eq!(parser.graph.graph_to_tuple(), graph_output);
}

#[test]
fn test_parser_simple2() {
    let input = r#"@func {}"#;
    let mut parser = parser::new("/Users/hannes.boerner/Downloads/criu-criu-dev".to_string());
    let parser_output = HashSet::from(["func".to_string()]);
    assert_eq!(parser.parse(input), parser_output);
    let graph_output = HashSet::from([("func".to_string(), "child1".to_string()), ("func".to_string(), "child2".to_string())]);
    assert_eq!(parser.graph.graph_to_tuple(), graph_output);
}

#[test]
fn test_parser() {
    let input = r#"{{@func}}"#;
    let mut parser = parser::new("/Users/hannes.boerner/Downloads/criu-criu-dev".to_string());
    let parser_output = HashSet::from(["parent1".to_string(), "parent2".to_string()]);
    assert_eq!(parser.parse(input), parser_output);
    let graph_output = HashSet::from([
        ("parent1".to_string(), "func".to_string()),
        ("parent1".to_string(), "parent1".to_string()),
        ("parent1".to_string(), "parent2".to_string()),
        ("parent2".to_string(), "func".to_string()),
        ("parent2".to_string(), "parent1".to_string()),
        ("parent2".to_string(), "parent2".to_string())]);
    assert_eq!(parser.graph.graph_to_tuple(), graph_output);
    //let g : tabbycat::Graph = parser.graph.try_into().unwrap();
    //assert_eq!(g.to_string(), "This test is unusable")
}