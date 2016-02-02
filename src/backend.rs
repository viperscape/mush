/// mush graph backend and abstraction to swap out with custom backend

use uuid::Uuid;
use std::collections::{HashMap,HashSet,VecDeque};

pub type Nid = Uuid; // graph node id
pub type Eid = (Nid,Nid); // graph edge id, (From,To)


/// unidirectional edge, use two edges for bidirectional/undirected graph
pub trait GraphEdge: Copy+Clone {
    fn default () -> Self;
}

pub trait GraphNode: Clone+EdgeGuard {
    //type P:Index<usize>; // position
    fn default () -> Self;
    
    fn get_base(&self) -> &NodeBase;
    fn get_base_mut(&mut self) -> &mut NodeBase;
    
    fn get_name(&self) -> &str;
    fn get_position(&self) -> &[f64;2]; //&Self::P;

    fn set_name(&mut self, s: &str);
    fn set_position(&mut self, p: [f64;2]); //Self::P);
}

/// trait specifying node connection requirements
pub trait EdgeGuard: PartialEq {
    fn guard(&self, node: &Self) -> bool;
}
//--

#[derive(Debug,Clone,PartialEq)]
pub struct NodeBase {
    nid: Nid,
    edges_to: HashSet<Nid>,
    edges_from: HashSet<Nid>,
}
impl NodeBase {
    pub fn new () -> NodeBase {
        NodeBase { nid: Uuid::new_v4(),
                   edges_to: HashSet::new(),
                   edges_from: HashSet::new(), }
    }
    /// direct the node towards another node
    // todo: rename me! sounds too similar to unidirectional
    fn direct (&mut self, to:&Nid) {
        self.edges_to.insert(*to);
    }
    fn direct_from (&mut self, from:&Nid) {
        self.edges_from.insert(*from);
    }
    fn undirect (&mut self, to:&Nid) {
        self.edges_to.remove(to);
    }
    fn undirect_from (&mut self, from:&Nid) {
        self.edges_from.remove(from);
    }
    pub fn get_edges(&self) -> Vec<Nid> {
        self.edges_to.iter().map(|nid| *nid).collect()
    }
    
    pub fn get_id(&self) -> Nid { self.nid }
}


pub trait Backend {
    type Node:GraphNode;
    type Edge:GraphEdge;
    type node_id;
    type edge_id;
    
    
    fn default() -> Self;
    fn get_node(&self, n: &Self::node_id) -> Option<&Self::Node>;
    fn get_node_mut(&mut self, n: &Self::node_id) -> Option<&mut Self::Node>;
    fn get_edge_mut (&mut self, e: &Self::edge_id) -> Option<&mut Self::Edge>;
    fn get_edge (&self, e: &Self::edge_id) -> Option<&Self::Edge>;
    fn new_node (&mut self) -> Self::node_id;
    fn add_node (&mut self, node: Self::Node) -> Self::node_id;
    fn remove(&mut self, n: &Self::node_id) -> Option<Self::Node>;
    fn direct(&mut self, from: &Self::node_id, to: &Self::node_id, e: Self::Edge) -> bool;
    fn undirect(&mut self, from: &Self::node_id, to: &Self::node_id);
    fn with_nodes<F:Fn(&Self::Node)>(&self, f: F);
    fn with_nodes_mut<F:FnMut(&mut Self::Node)>(&mut self, f: F);
}

//----
#[derive(Debug)]
pub struct Graph<E:GraphEdge, N:GraphNode> {
    nodes: HashMap<Nid,N>,
    edges: HashMap<Eid,E>,
    
    // todo: as traits
    is_weighted: bool,
    is_directed: bool,
    is_tracking: bool,  // tracking from-edges
}

// todo: work on getting this to work along side of regular graph direct method
/*impl<E:GraphEdge, N:GraphNode> Graph<E,N> where N: EdgeGuard {
    pub fn direct(&mut self, from: &Nid, to: &Nid, e: E) -> bool {
        if let Some(f) = self.nodes.get(from) {
            if let Some(t) = self.nodes.get(to) {
                if !f.guard(t) { return false }
            }
        }

        true
    }
}*/

impl<E:GraphEdge, N:GraphNode> Backend for Graph<E,N> {
    type Node = N;
    type Edge = E;
    type node_id = Nid;
    type edge_id = Eid;
    fn default() -> Graph<E,N> {
        Graph { nodes: HashMap::new(),
                edges: HashMap::new(),
                is_weighted: false,
                is_directed: true,
                is_tracking: false, }
    }

    fn with_nodes<F:Fn(&N)>(&self, f: F) {
        for (_,n) in self.nodes.iter() {
            f(n);
        }
    }
    fn with_nodes_mut<F:FnMut(&mut N)>(&mut self, mut f: F) {
        for (_,n) in self.nodes.iter_mut() {
            f(n);
        }
    }
    
    
    /// manual accessors
    fn get_node_mut(&mut self, n: &Nid) -> Option<&mut N> {
        self.nodes.get_mut(n)
    }
    fn get_node(&self, n: &Nid) -> Option<&N> {
        self.nodes.get(n)
    }

    fn get_edge_mut (&mut self, e: &Eid) -> Option<&mut E> {
        self.edges.get_mut(e)
    }
    fn get_edge (&self, e: &Eid) -> Option<&E> {
        self.edges.get(e)
    }

    fn new_node (&mut self) -> Nid { //todo: maybe_edges fn arg
        let n: N = GraphNode::default();
        let nid = n.get_base().nid;
        self.nodes.insert(nid,n);
        nid
    }

    fn add_node (&mut self, node: N) -> Nid {
        let n: N = node;
        let nid = n.get_base().nid;
        self.nodes.insert(nid,n);
        nid
    }
    
    fn remove(&mut self, n: &Nid) -> Option<N> {
        self.nodes.remove(n)
    }

    //todo: check for previous edge!
    fn direct(&mut self, from: &Nid, to: &Nid, e: E) -> bool {
        let mut r = false;

        if let Some(f) = self.nodes.get(from) {
            if let Some(t) = self.nodes.get(to) {
                if !f.guard(t) { return false }
            }
        }
        
        let eid = self.add_edge(from,to,e);

        if !self.get_node(to).is_some() { return false }
        
        if let Some(f) = self.nodes.get_mut(from) {
            f.get_base_mut().direct(to);
            r = true;
        }

        if r {
            
            
            if self.is_tracking { let t = self.nodes.get_mut(to).unwrap();
                                  t.get_base_mut().direct_from(from); }
            
            if !self.is_directed {
                if let Some(t) = self.nodes.get_mut(to) {
                    t.get_base_mut().direct(to);
                }
                else { r = false; }
            }
        }

        if !r { self.edges.remove(&eid); }
        
        r
    }
    
    fn undirect(&mut self, from: &Nid, to: &Nid) {
        let eid = (*from,*to);
        
        if let Some(f) = self.nodes.get_mut(from) {
            f.get_base_mut().undirect(to);
        }
        
        if self.is_tracking {  let t = self.nodes.get_mut(to).unwrap();
                               t.get_base_mut().undirect_from(from); }
        if !self.is_directed {
            if let Some(t) = self.nodes.get_mut(to) {
                t.get_base_mut().undirect(from);
            }
        }

        self.edges.remove(&eid);
    }

}

//todo: turn many of these methods into a trait
impl<E:GraphEdge, N:GraphNode> Graph<E,N> {
    fn add_edge (&mut self, from: &Nid, to: &Nid, e: E) -> Eid {
        let eid = (*from,*to);
        self.edges.insert(eid,e);
        eid
    }

    // search functions
    // todo: consider weights between nodes to direct search
    pub fn get_path(&self, s: GraphSearch) -> Option<Vec<Nid>> {
        let mut visited = HashSet::new();
        let mut result = vec!();
        
        match s {
            GraphSearch::Depth(from,to) => {
                let mut stack = vec!();

                stack.push(from);
                
                while stack.len() > 0 {
                    let cursor = *stack.last().unwrap();
                    visited.insert(cursor);
                    
                    if let Some(ref node) = self.get_node(&cursor) {

                        //get first unvisited node
                        let not_visited = node.get_base().edges_to.iter().find(|&n| !visited.contains(n));
                        
                        if let Some(&n) = not_visited {
                            if !self.is_tracking || self.nodes.contains_key(&n) { 
                                stack.push(n); //add node to check
                                result.push(n);

                                if let Some(to_node) = to {
                                    if n == to_node { break; }
                                }
                            }
                        }
                        else { stack.pop(); } //nothing left, pop off and head back a node
                    }
                    else { stack.pop(); } //invalid node?
                }

                if let Some(to_node) = to {
                    if result.contains(&to_node) {
                        return Some(result)
                    }
                }

                return None
            },
            GraphSearch::Breadth(from,to) => { // breadth first search, uses a queue
                let mut queue = VecDeque::new();

                queue.push_back(from);
                visited.insert(from);
                result.push(from);

                while queue.len() > 0 {
                    let cursor = *queue.front().unwrap();
                    if let Some(ref node) = self.get_node(&cursor) {
                        //get unvisted nodes to queue up
                        let not_visited: Vec<Option<Nid>> = node.get_base().edges_to.iter().map(|&n| {
                            if !visited.contains(&n) {
                                Some(n)
                            }
                            else { None }
                        }).collect();

                        for maybe_node in not_visited {
                            if let Some(n) = maybe_node {
                                if !self.is_tracking || self.nodes.contains_key(&n) { //node exists?
                                    queue.push_back(n);
                                    visited.insert(n);
                                    result.push(n);

                                    if let Some(to_node) = to {
                                        if n == to_node { break; }
                                    }
                                }
                            }
                        }

                        queue.pop_front();
                    }
                    else { queue.pop_front(); }
                }

                if let Some(to_node) = to {
                    if result.contains(&to_node) {
                        return Some(result)
                    }
                }

                return None
            },
            _ => None, // todo: djk algo
        }
    }

    //this is virtually the same as get_path dfs, should abstract dfs somehow to use it for this
    pub fn get_cycle(&self, from: Nid) -> HashSet<(Nid,Nid)> {
        let mut stack = vec!();
        let mut visited = HashSet::new();
        let mut r = HashSet::new(); //Vec::new();

        stack.push(from);
        
        while stack.len() > 0 {
            let cursor = *stack.last().unwrap();
            visited.insert(cursor);
            
            if let Some(ref node) = self.get_node(&cursor) {
                
                //does the cursor point to a node on stack
                for n in node.get_base().edges_to.iter() {
                    if stack.contains(&n) {
                        r.insert((*n,cursor));
                    }
                }

                //get first unvisited node
                let not_visited = node.get_base().edges_to.iter().find(|n| !visited.contains(n));
                
                if let Some(&n) = not_visited {
                    if !stack.contains(&n) {
                        if !self.is_tracking || self.nodes.contains_key(&n) {
                            stack.push(n); //add node to check
                        }
                    }
                }
                else { stack.pop(); } //nothing left, pop off and head back a node
            }
            else { stack.pop(); } //invalid node?
        }

        return r
    }

    /// get all nodes in their graph layout (DFS)
    pub fn get_all_nodes(&self) -> Vec<Vec<Nid>> {
        let mut result = vec!();
        let mut visited = HashSet::new();
        
        for (n,_) in &self.nodes {
            if !visited.contains(n) {
                if let Some(r) = self.get_path(GraphSearch::Depth(*n,None)) {
                    for _n in r.iter() { visited.insert(*_n); }
                    result.push(r);
                }
            }
        }
        result
    }

    /// get immediate next node from list of connected nodes for the current node
    pub fn get_next(&self, from: &Nid) -> Option<Nid> {
        if let Some(n) = self.nodes.get(from) {
            if let Some(next_id) = n.get_base().edges_to.iter().next() {
                if !self.is_tracking || self.nodes.contains_key(&next_id) {
                    return Some(*next_id) // grab uuid key
                }
            }
        }
        None
    }

    // TODO: implement me!
    #[allow(dead_code)]
    fn is_connected() -> bool { false }
    #[allow(dead_code)]
    fn is_complete() -> bool { false }
    #[allow(dead_code)]
    fn get_path_shortest(&self) -> bool {
        if !self.is_weighted { false } //must be weighted
        else { false } //todo: use bfs
    }
}

// todo: impl as trait instead?
pub enum GraphSearch {
    Depth(Nid,Option<Nid>), // used on part of graph for reachability, and all of graph for cycle-detection
    Breadth(Nid,Option<Nid>), // used on part of graph for reachability, and (unweighted) for shortest path
    Dijkstra(Nid,Nid), // used on part of graph (weighted) for shortest path
}

/*impl<'a, E:GraphEdge, N:GraphNode> Iterator for Graph<E,N> {
    type Item = &'a N;
    fn next(&mut self) -> Option<Self::Item> {
        self.nodes.values().next()
    }
}*/

pub struct GraphBuilder<E:GraphEdge,N:GraphNode> (Graph<E,N>);

/// tracking specifies that you wish to track the from-node connections
impl<E:GraphEdge, N:GraphNode> GraphBuilder<E,N> {
    pub fn new() -> GraphBuilder<E,N> {
        GraphBuilder(Graph::default())
    }

    pub fn directed(mut self, d: bool) -> GraphBuilder<E,N> {
        self.0.is_directed = d;
        self
    }
    pub fn weighted(mut self, w: bool) -> GraphBuilder<E,N> {
        self.0.is_weighted = w;
        self
    }
    pub fn tracking(mut self, t: bool) -> GraphBuilder<E,N> {
        self.0.is_tracking = t;
        self
    }
    pub fn build(self) -> Graph<E,N> {
        self.0
    }
}
