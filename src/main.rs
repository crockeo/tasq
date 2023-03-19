use std::fs::File;
use std::process::Stdio;
use std::rc::Rc;

use chrono::DateTime;
use chrono::Utc;
use structopt::StructOpt;
use uuid::Uuid;

use db::Node;
use db::NodeID;

mod db;
mod ui;

fn main() -> anyhow::Result<()> {
    async_std::task::block_on(async_main())
}

async fn async_main() -> anyhow::Result<()> {
    let opt = Opt::from_args();

    let database = db::Database::default_new().await?;
    match opt {
        Opt::Add(args) => add(args, database).await,
        Opt::Connect(args) => connect(args, database).await,
        Opt::Edit(args) => edit(args, database).await,
        Opt::Show(args) => show(args, database).await,
        Opt::Next(args) => next(args, database).await,
        Opt::UI => ui::main(database).await,
    }?;
    Ok(())
}

async fn add(args: AddArgs, database: db::Database) -> anyhow::Result<()> {
    let mut node = Node::new();
    if let Some(title) = args.title {
        node.title = title;
    }
    if let Some(description) = args.description {
        node.description = description;
    }
    if let Some(scheduled) = args.scheduled {
        node.scheduled = Some(DateTime::from_local(scheduled, Utc));
    }
    if let Some(due) = args.due {
        node.due = Some(DateTime::from_local(due, Utc));
    }
    println!("{}", node.id.to_string());
    database.add(&node).await?;
    Ok(())
}

async fn connect(args: ConnectArgs, database: db::Database) -> anyhow::Result<()> {
    database.connect(args.from, args.to).await?;
    Ok(())
}

async fn edit(args: EditArgs, database: db::Database) -> anyhow::Result<()> {
    let editor = match std::env::var("EDITOR") {
        Err(_) => "vi".to_string(),
        Ok(editor) => editor,
    };

    let node = database.get_node(args.node).await?;

    let temp_dir = tempfile::tempdir()?;
    let mut filename = temp_dir.path().to_path_buf();
    filename.push(format!("{}.json", node.id));

    {
        let file = File::create(&filename)?;
        serde_json::to_writer_pretty::<_, Node>(file, &node)?;
    };

    let status = std::process::Command::new(editor)
        .arg(&filename)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()?;

    // If we fail to edit, just don't modify the database.
    if !status.success() {
        return Ok(());
    }

    let node = {
        let file = File::open(&filename)?;
        serde_json::from_reader::<_, Rc<Node>>(file)?
    };
    database.update(&node).await?;

    Ok(())
}

async fn show(args: ShowArgs, database: db::Database) -> anyhow::Result<()> {
    let to_show = if let Some(root_id) = args.root {
        vec![root_id]
    } else {
        database.get_roots().await?
    };

    for root in to_show.into_iter() {
        let mut dfs = database.dfs(root).await?;
        while let Some((node, depth)) = dfs.next().await? {
            for _ in 0..2 * depth {
                print!(" ");
            }
            println!("{}", database.get_node(node).await?.short_repr());
        }
    }

    Ok(())
}

async fn next(args: NextArgs, database: db::Database) -> anyhow::Result<()> {
    let to_next = if let Some(root_id) = args.root {
        vec![root_id]
    } else {
        database.get_roots().await?
    };

    for root in to_next.into_iter() {
        let mut dfs = database.dfs(root).await?;
        while let Some((node, _)) = dfs.next().await? {
            if !database.has_children(node).await? {
                println!("{}", database.get_node(node).await?.short_repr());
            }
        }
    }

    Ok(())
}

#[derive(Debug, StructOpt)]
enum Opt {
    Add(AddArgs),
    Connect(ConnectArgs),
    Edit(EditArgs),
    Show(ShowArgs),
    Next(NextArgs),
    UI,
}

#[derive(Debug, StructOpt)]
struct AddArgs {
    #[structopt(short = "t", long = "title")]
    title: Option<String>,
    #[structopt(short = "d", long = "description")]
    description: Option<String>,
    #[structopt(short = "s", long = "scheduled")]
    scheduled: Option<chrono::NaiveDateTime>,
    #[structopt(short = "e", long = "due")]
    due: Option<chrono::NaiveDateTime>,
}

#[derive(Debug, StructOpt)]
struct ConnectArgs {
    from: Uuid,
    to: Uuid,
}

#[derive(Debug, StructOpt)]
struct EditArgs {
    node: NodeID,
}

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
