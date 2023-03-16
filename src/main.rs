use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::path::Path;
use std::rc::Rc;

use structopt::StructOpt;

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
    let mut graph_file = std::env::current_dir()?;
    graph_file.push("graph.json");

    let mut graph = Graph::load(&graph_file)?;

    let opt = Opt::from_args();
    graph = match opt {
        Opt::Add(args) => add(args, graph),
	Opt::Connect(args) => connect(args, graph),
	Opt::Edit(args) => edit(args, graph),
	Opt::Show(args) => show(args, graph),
	Opt::Next(args) => next(args, graph),
    }?;

    graph.save(&graph_file)?;
    Ok(())
}

fn add(args: AddArgs, graph: Graph) -> anyhow::Result<Graph> {
    todo!()
}

fn connect(args: ConnectArgs, graph: Graph) -> anyhow::Result<Graph> {
    todo!()
}

fn edit(args: EditArgs, graph: Graph) -> anyhow::Result<Graph> {
    todo!()
}

fn show(args: ShowArgs, graph: Graph) -> anyhow::Result<Graph> {
    todo!()
}

fn next(args: NextArgs, graph: Graph) -> anyhow::Result<Graph> {
    todo!()
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
struct ConnectArgs {}

#[derive(Debug, StructOpt)]
struct EditArgs {}

#[derive(Debug, StructOpt)]
struct ShowArgs {}

#[derive(Debug, StructOpt)]
struct NextArgs {}

type NodeID = usize;

pub struct Node {
    pub id: NodeID,
    pub title: String,
    pub description: String,
    pub scheduled: Option<()>,
    pub due: Option<()>,
}

pub struct Graph {
    nodes: BTreeMap<NodeID, Rc<Node>>,
    roots: BTreeSet<NodeID>,
    edges: BTreeMap<NodeID, NodeID>,
}

impl Graph {
    pub fn load(path: &Path) -> anyhow::Result<Graph> {
        todo!()
    }

    pub fn save(&self, path: &Path) -> anyhow::Result<()> {
        todo!()
    }

    pub fn add(&mut self, node: Node) {
        let node = Rc::new(node);
        self.roots.insert(node.id);
        self.nodes.insert(node.id, node);
    }

    pub fn connect(&mut self, from: NodeID, to: NodeID) {
        self.edges.insert(from, to);
        self.roots.remove(&to);
    }

    pub fn get_roots(&self) -> Vec<Rc<Node>> {
        self.roots
            .iter()
            .map(|root| self.nodes.get(root).expect("missing node for root").clone())
            .collect()
    }
}
