use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::fs::File;
use std::io;
use std::path::Path;
use std::path::PathBuf;
use std::rc::Rc;

use anyhow::anyhow;
use serde::Deserialize;
use serde::Serialize;
use structopt::StructOpt;
use uuid::Uuid;

// CLI interface to add/remove tasks from a graph database
// and then find the next thing you should do for a particular set of tasks
//
// UI experience
// - make node
// - connect node to other node (unidirectional)
// - edit node
// - visualize a DAG
// - find the "next" (or set of next tasks) for a task

fn main() -> anyhow::Result<()> {
    let opt = Opt::from_args();

    let mut graph = Graph::load_default()?;
    graph = match opt {
        Opt::Add(args) => add(args, graph),
        Opt::Connect(args) => connect(args, graph),
        Opt::Edit(args) => edit(args, graph),
        Opt::Show(args) => show(args, graph),
        Opt::Next(args) => next(args, graph),
    }?;
    graph.save_default()
}

fn add(args: AddArgs, mut graph: Graph) -> anyhow::Result<Graph> {
    let node = Node::new();
    println!("{}", node.id.to_string());
    graph.add(node);
    Ok(graph)
}

fn connect(args: ConnectArgs, mut graph: Graph) -> anyhow::Result<Graph> {
    graph.connect(args.from, args.to)?;
    Ok(graph)
}

fn edit(args: EditArgs, graph: Graph) -> anyhow::Result<Graph> {
    todo!()
}

fn show(args: ShowArgs, graph: Graph) -> anyhow::Result<Graph> {
    let to_show = if let Some(root_id) = args.root {
        vec![root_id]
    } else {
        graph.get_roots()
    };

    for root in to_show.into_iter() {
	for (node, depth) in graph.dfs(root)? {
	    for _ in 0..2*depth {
		print!(" ");
	    }
	    println!("{}", graph.get_node(node)?.short_repr());
	}
    }

    Ok(graph)
}

fn next(args: NextArgs, graph: Graph) -> anyhow::Result<Graph> {
    let to_next = if let Some(root_id) = args.root {
        vec![root_id]
    } else {
        graph.get_roots()
    };

    for root in to_next.into_iter() {
	for (node, _) in graph.dfs(root)? {
	    if !graph.has_children(node)? {
		println!("{}", graph.get_node(node)?.short_repr());
	    }
	}
    }

    Ok(graph)
}

#[derive(Debug, StructOpt)]
enum Opt {
    Add(AddArgs),
    Connect(ConnectArgs),
    Edit(EditArgs),
    Show(ShowArgs),
    Next(NextArgs),
}

#[derive(Debug, StructOpt)]
struct AddArgs {}

#[derive(Debug, StructOpt)]
struct ConnectArgs {
    from: Uuid,
    to: Uuid,
}

#[derive(Debug, StructOpt)]
struct EditArgs {}

#[derive(Debug, StructOpt)]
struct ShowArgs {
    #[structopt(short = "r", long = "root")]
    root: Option<NodeID>,
}

#[derive(Debug, StructOpt)]
struct NextArgs {
    #[structopt(short = "r", long = "root")]
    root: Option<NodeID>,
}

type NodeID = Uuid;

#[derive(Deserialize, Serialize)]
pub struct Node {
    pub id: NodeID,
    pub title: String,
    pub description: String,
    pub scheduled: Option<()>,
    pub due: Option<()>,
}

impl Node {
    pub fn new() -> Self {
        Node {
            id: Uuid::new_v4(),
            title: "".to_string(),
            description: "".to_string(),
            scheduled: None,
            due: None,
        }
    }

    pub fn short_repr(&self) -> String {
        self.id.to_string()
    }
}

#[derive(Default, Deserialize, Serialize)]
pub struct Graph {
    nodes: BTreeMap<NodeID, Rc<Node>>,
    roots: BTreeSet<NodeID>,
    edges: BTreeMap<NodeID, BTreeSet<NodeID>>,
    reverse_edges: BTreeMap<NodeID, BTreeSet<NodeID>>,
}

impl Graph {
    pub fn load_default() -> anyhow::Result<Graph> {
        Self::load(&Self::default_path()?)
    }

    pub fn load(path: &Path) -> anyhow::Result<Graph> {
        let file = match File::open(path) {
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Self::default()),
            x => x,
        }?;
        Ok(serde_json::from_reader::<_, Graph>(file)?)
    }

    pub fn save_default(&self) -> anyhow::Result<()> {
        self.save(&Self::default_path()?)
    }

    pub fn save(&self, path: &Path) -> anyhow::Result<()> {
        let file = File::create(path)?;
        serde_json::to_writer(file, self)?;
        Ok(())
    }

    pub fn add(&mut self, node: Node) {
        let node = Rc::new(node);
        self.roots.insert(node.id);
        self.nodes.insert(node.id, node);
    }

    pub fn connect(&mut self, from: NodeID, to: NodeID) -> anyhow::Result<()> {
        if !self.nodes.contains_key(&from) {
            return Err(anyhow!("Missing node {}", from.to_string()));
        }
        if !self.nodes.contains_key(&to) {
            return Err(anyhow!("Missing node {}", to.to_string()));
        }

        if !self.edges.contains_key(&from) {
            self.edges.insert(from, BTreeSet::default());
        }
        self.edges.get_mut(&from).unwrap().insert(to);
        self.roots.remove(&to);
        Ok(())
    }

    pub fn get_node(&self, id: NodeID) -> anyhow::Result<Rc<Node>> {
        Ok(self
            .nodes
            .get(&id)
            .ok_or(anyhow!("Missing node ID {}", id))?
            .clone())
    }

    pub fn has_children(&self, id: NodeID) -> anyhow::Result<bool> {
	self.exist_check(&id)?;
	Ok(self.edges.contains_key(&id) && self.edges[&id].len() > 0)
    }

    pub fn get_children(&self, id: NodeID) -> anyhow::Result<BTreeSet<Uuid>> {
	self.exist_check(&id)?;
        let Some(children) = self.edges.get(&id) else {
	    return Ok(BTreeSet::default());
	};
        Ok(children.clone())
    }

    pub fn get_roots(&self) -> Vec<NodeID> {
        self.roots.iter().map(Uuid::clone).collect()
    }

    pub fn dfs(&self, root: NodeID) -> anyhow::Result<DFSIter<'_>> {
        if !self.nodes.contains_key(&root) {
            return Err(anyhow!("Missing node {}", root));
        }

        Ok(DFSIter {
            graph: self,
            seen: BTreeSet::default(),
            stack: vec![(root, 0)],
        })
    }

    fn exist_check(&self, id: &NodeID) -> anyhow::Result<()> {
	if !self.nodes.contains_key(id) {
	    return Err(anyhow!("Missing node {}", id));
	}
	Ok(())
    }

    fn default_path() -> anyhow::Result<PathBuf> {
        let mut graph_file = std::env::current_dir()?;
        graph_file.push("graph.json");
        Ok(graph_file)
    }
}

pub struct DFSIter<'a> {
    graph: &'a Graph,
    seen: BTreeSet<NodeID>,
    stack: Vec<(NodeID, usize)>,
}

impl<'a> Iterator for DFSIter<'a> {
    type Item = (NodeID, usize);

    fn next(&mut self) -> Option<Self::Item> {
        let (next, depth) = self.stack.pop()?;
        self.seen.insert(next.clone());
        for child in self.graph.get_children(next).unwrap() {
            if self.seen.contains(&child) {
                continue;
            }
            self.stack.push((child, depth + 1));
        }
        Some((next, depth))
    }
}
