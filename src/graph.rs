use std::{collections::{HashMap, HashSet, BTreeMap}, fmt::Display, rc::Rc};

use itertools::Itertools;

pub type NodeId = u64;
pub type NodeSet = HashSet<NodeId>;

#[derive(Debug, Clone)]
pub struct Arrow {
    pub from: NodeId,
    pub to: NodeId,
}

#[derive(Debug)]
pub struct Node<T> {
    incoming: Vec<Arrow>,
    outgoing: Vec<Arrow>,
    value: Rc<T>,
    id: NodeId,
}

impl<T> std::clone::Clone for Node<T> {
    fn clone(&self) -> Self {
        Self {
            incoming: self.incoming.clone(),
            outgoing: self.outgoing.clone(),
            value: self.value.clone(),
            id: self.id,
        }
    }
}

impl<T> Node<T> {
    pub fn new(value: T, id: NodeId) -> Self {
        Self {
            value: value.into(),id, incoming: Vec::new(), outgoing: Vec::new(),
        }
    }
    pub fn link(&mut self, other: &mut Self) -> &mut Self {
        self.outgoing.push(Arrow {
            from: self.id,
            to: other.id,
        });
        other.incoming.push(Arrow {
            from: self.id,
            to: other.id,
        });
        self
    }
    pub fn id(&self) -> NodeId {
        self.id
    }
    pub fn outgoing(&self) -> &[Arrow] {
        &self.outgoing
    }
    pub fn incoming(&self) -> &[Arrow] {
        &self.incoming
    }
    pub fn value(&self) -> &T {
        &self.value
    }
}

#[derive(Debug)]
pub struct Graph<T> {
    nodes: HashMap<NodeId, Node<T>>,
    current_id: NodeId,
}

macro_rules! set {
    ($($e:expr),*) => {{
        let mut s = HashSet::new();
        $(s.insert($e);)*
        s
    }};
}

impl<T> std::ops::Index<NodeId> for Graph<T> {
    type Output = Node<T>;
    fn index(&self, index: NodeId) -> &Self::Output {
        self.nodes.get(&index).unwrap()
    }
}

impl<T> std::ops::IndexMut<NodeId> for Graph<T> {
    fn index_mut(&mut self, index: NodeId) -> &mut Self::Output {
        self.nodes.get_mut(&index).unwrap()
    }
}

impl<T> Graph<T> {
    pub fn new() -> Self {
        Self {
            nodes: Default::default(),
            current_id: 0,
        }
    }

    pub fn new_node(&mut self, value: T) -> NodeId {
        let id = self.current_id;
        self.nodes.insert(id, Node::new(value, id));
        self.current_id += 1;
        id
    }

    pub fn new_nodes<const N: usize>(&mut self, values: [T; N]) -> [NodeId; N] {
        values.map(|value| self.new_node(value))
    }

    pub fn link(&mut self, from: NodeId, to: NodeId) {
        let [from, to] = self.nodes.get_many_mut([&from, &to]).unwrap();
        from.link(to);
    }

    pub fn iter(&self) -> impl Iterator<Item = &Node<T>> {
        self.nodes.values()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut Node<T>> {
        self.nodes.values_mut()
    }

    fn _walk(&self, id: NodeId, visited: &mut NodeSet, mut f: &mut impl FnMut(&Node<T>)) {
        if visited.contains(&id) {
            return
        }
        let node = &self.nodes[&id];
        f(&node);
        visited.insert(id);
        for arrow in &node.outgoing {
            self._walk(arrow.to, visited, f);
        }
    }

    pub fn walk(&self, mut f: impl FnMut(&Node<T>)) {
        let mut visited = NodeSet::new();
        match self.nodes.keys().next() {
            Some(current) => self._walk(*current, &mut visited, &mut f),
            None => ()
        }
    }

    pub fn nplus(&self, nodeset: &NodeSet) -> NodeSet {
        let mut nplus = NodeSet::new();
        for n in nodeset.iter() {
            for arrow in self.nodes[n].outgoing.iter() {
                nplus.insert(arrow.to);
            }
        }
        nplus
    }

    pub fn nminus(&self, nodeset: &NodeSet) -> NodeSet {
        let mut nminus = NodeSet::new();
        for n in nodeset.iter() {
            for arrow in self.nodes[n].incoming.iter() {
                nminus.insert(arrow.from);
            }
        }
        nminus
    }

    pub fn subgraph_of(&self, nodeset: &NodeSet) -> Self {
        let mut g = Self::new();
        for n in nodeset.iter() {
            let mut node = self.nodes[n].clone();
            node.outgoing.retain(|arrow| nodeset.contains(&arrow.to));
            node.incoming.retain(|arrow| nodeset.contains(&arrow.to));
            g.nodes.insert(node.id, node);
        }
        g
    }

    pub fn f_conex(&self) -> Vec<Self> {
        let mut w = vec![];
        let mut unvisited: HashSet<_> = self.iter().map(Node::id).collect();
        while let Some(node) = unvisited.iter().next() {
            let mut wk;
            let mut rplus = set![*node];
            let mut rminus = set![*node];
            loop {
                let nplus_diff: NodeSet = self.nplus(&rplus).difference(&rplus).copied().collect();
                if nplus_diff.is_empty() {
                    break;
                }
                wk = nplus_diff;
                rplus.extend(&wk);
            }
            loop {
                let nminus_diff: NodeSet = self.nminus(&rminus).difference(&rminus).copied().collect();
                if nminus_diff.is_empty() {
                    break;
                }
                wk = nminus_diff;
                rminus.extend(&wk);
            }
            wk = rplus.intersection(&rminus).copied().collect();
            unvisited.retain(|n| !wk.contains(n));
            w.push(self.subgraph_of(&wk));
        }
        w
    }
}

impl<T: Eq> Graph<T> {
    pub fn new_node_or_get(&mut self, value: T) -> NodeId {
        self.find(&value).unwrap_or_else(|| self.new_node(value))
    }
    pub fn find(&self, value: &T) -> Option<NodeId> {
        self.iter().find(|node| *node.value == *value).map(|node| node.id)
    }
}

impl<T: Display> Graph<T> {
    pub fn to_dot(&self) -> String {
        let mut nodes = self.iter()
            .sorted_by_cached_key(|node| node.id())
            .map(|node| {
                let mut s = vec![];
                s.push(format!("{} [label=\"{}\"];", node.id, node.value));
                for arrow in &node.outgoing {
                    s.push(format!("{} -> {};", arrow.from, arrow.to));
                }
                s
            })
            .flatten()
            .collect::<Vec<_>>();
        nodes.insert(0, "digraph {".to_string());
        nodes.push("}".to_string());
        nodes.join("\n")
    }
}
