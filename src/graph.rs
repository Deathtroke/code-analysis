use std::collections::HashSet;
use petgraph::graph::NodeIndex;

pub struct Graph {
    pub pet_graph: petgraph::Graph<String, ()>,
    pub(crate) nodes: HashSet<Node>,
}

#[derive(Hash, Eq, PartialEq, Debug, Clone)]
pub struct Node {
    pub name: String,
    pub priority: u32,
}

impl Graph {
    pub fn graph_to_tuple(&self) -> HashSet<(String, String)>{
        let mut result :HashSet<(String, String)> = HashSet::new();
        for edge in self.pet_graph.raw_edges() {
            let mut target = "".to_string();
            if self.pet_graph.node_weight(edge.target()).is_some(){
                target = self.pet_graph.node_weight(edge.target()).unwrap().to_owned();
            }
            result.insert((self.pet_graph.node_weight(edge.source()).unwrap().to_owned(), target));
        }
        result
    }

    pub fn add_node(&mut self, node_name: String, prio: u32) {
        let mut node_exists = false;
        for node in self.pet_graph.node_indices(){
            if self.pet_graph.node_weight(node).unwrap().to_owned() == node_name {
                node_exists = true;
            }
        }
        if !node_exists {
            self.pet_graph.add_node(node_name.clone());
            let node = Node{name: node_name.clone(), priority: prio};
            self.nodes.insert(node);
        }
    }

    pub fn add_edge(&mut self, start: String, end : String) -> bool {
        let mut edge_exists = false;
        let mut start_node: NodeIndex = NodeIndex::new(0);
        let mut end_node: NodeIndex = NodeIndex::new(0);
        for node in self.pet_graph.node_indices(){
            if self.pet_graph.node_weight(node).is_some() {
                let node_name = self.pet_graph.node_weight(node).unwrap().clone();
                if node_name == start {
                    start_node = node;
                }
                if node_name == end {
                    end_node = node;
                }
            }
        }
        if self.pet_graph.contains_edge(start_node, end_node) {
            edge_exists = true;
        }
        if !edge_exists {
            self.pet_graph.add_edge(start_node, end_node, ());
            for node in self.nodes.clone() {
                if (node.name == end.clone()) {
                    let new_prio = node.priority + 1;
                    let new_node = Node{name: node.name.clone(), priority: new_prio};
                    self.nodes.remove(&node.clone());
                    self.nodes.insert(new_node);
                }
            }
        }
        self.pet_graph.neighbors(end_node).next().is_some()
    }

    pub fn graph_to_dot(&mut self) -> String {
        format!("{:?}",  petgraph::dot::Dot::new(&self.pet_graph.clone()))
    }
}
/*
impl TryFrom<Graph> for tabbycat::Graph {
    type Error = anyhow::Error;

    fn try_from(g: Graph) -> Result<Self, Self::Error> {
        let mut stmts = tabbycat::StmtList::new();

        for edge in &g.edges {
            if edge.node_to.is_some() {
                stmts = stmts.add_edge(
                    tabbycat::Edge::head_node(tabbycat::Identity::id(edge.node_from.as_str())?, None)
                        .arrow_to_node(tabbycat::Identity::id(edge.node_to.as_ref().unwrap().as_str())?, None),
                );
            } else {
                stmts = stmts.add_edge(
                    tabbycat::Edge::head_node(tabbycat::Identity::id(edge.node_from.as_str())?, None)
                );
            }
        }

        tabbycat::GraphBuilder::default()
            .graph_type(tabbycat::GraphType::DiGraph)
            .strict(false)
            .id(tabbycat::Identity::id("G").unwrap())
            .stmts(stmts)
            .build()
            .map_err(anyhow::Error::msg)
    }
}*/
