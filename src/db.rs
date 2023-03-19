use std::collections::BTreeSet;
use std::fs::File;
use std::path::Path;

use anyhow::anyhow;
use chrono::serde::ts_seconds_option;
use chrono::DateTime;
use chrono::LocalResult;
use chrono::TimeZone;
use chrono::Utc;
use serde::Deserialize;
use serde::Serialize;
use sqlx::sqlite::SqliteConnectOptions;
use sqlx::sqlite::SqlitePool;
use sqlx::sqlite::SqlitePoolOptions;
use sqlx::sqlite::SqliteRow;
use sqlx::Row;
use uuid::Uuid;

pub struct Database {
    pool: SqlitePool,
}

impl Database {
    pub async fn default_new() -> anyhow::Result<Self> {
        let mut graph_file = std::env::current_dir()?;
        graph_file.push("graph.sqlite3");
        Self::new(&graph_file).await
    }

    pub async fn new(path: impl AsRef<Path>) -> anyhow::Result<Self> {
        let path = path.as_ref();
        let mut create_tables = false;
        if !path.exists() {
            create_tables = true;
            File::create(path)?;
        }

        let pool = SqlitePoolOptions::new()
            .connect_with(SqliteConnectOptions::new().filename(path))
            .await?;

        let db = Self { pool };
        if create_tables {
            db.create_nodes_table().await?;
            db.create_edges_table().await?;
        }

        Ok(db)
    }

    async fn create_nodes_table(&self) -> anyhow::Result<()> {
        let contents = std::include_str!("sql/create_nodes.sql");
        sqlx::query(contents)
            .execute(&mut self.pool.acquire().await?)
            .await?;
        Ok(())
    }

    async fn create_edges_table(&self) -> anyhow::Result<()> {
        let contents = std::include_str!("sql/create_edges.sql");
        sqlx::query(contents)
            .execute(&mut self.pool.acquire().await?)
            .await?;
        Ok(())
    }

    pub async fn add(&self, node: &Node) -> anyhow::Result<()> {
        let query_str = std::include_str!("sql/insert_node.sql");
        let query = sqlx::query(query_str)
            .bind(node.id.to_string())
            .bind(&node.title)
            .bind(&node.description)
            .bind(node.scheduled.map(|dt| dt.timestamp_millis()))
            .bind(node.due.map(|dt| dt.timestamp_millis()));
        query.execute(&mut self.pool.acquire().await?).await?;
        Ok(())
    }

    pub async fn update(&self, node: &Node) -> anyhow::Result<()> {
        self.exists_check(&node.id).await?;

        let query_str = std::include_str!("sql/insert_node.sql");
        let query = sqlx::query(query_str)
            .bind(&node.title)
            .bind(&node.description)
            .bind(node.scheduled.map(|dt| dt.timestamp_millis()))
            .bind(node.due.map(|dt| dt.timestamp_millis()))
            .bind(node.id.to_string());
        query.execute(&mut self.pool.acquire().await?).await?;
        Ok(())
    }

    pub async fn connect(&self, from: NodeID, to: NodeID) -> anyhow::Result<()> {
        self.exists_check(&from).await?;
        self.exists_check(&to).await?;

        let query_str = std::include_str!("sql/connect_nodes.sql");
        let query = sqlx::query(query_str)
            .bind(from.to_string())
            .bind(to.to_string());
        query.execute(&mut self.pool.acquire().await?).await?;
        Ok(())
    }

    pub async fn get_node(&self, id: NodeID) -> anyhow::Result<Node> {
        let row = sqlx::query("SELECT * FROM nodes WHERE uuid = ?")
            .bind(id.to_string())
            .fetch_one(&mut self.pool.acquire().await?)
            .await?;
        row.try_into()
    }

    pub async fn has_children(&self, id: NodeID) -> anyhow::Result<bool> {
        let count = sqlx::query("SELECT COUNT(*) FROM edges WHERE from_uuid = ?")
            .bind(id.to_string())
            .fetch_one(&mut self.pool.acquire().await?)
            .await?;
        let count: i64 = count.get(0);
        Ok(count > 0)
    }

    pub async fn get_children(&self, id: NodeID) -> anyhow::Result<Vec<Uuid>> {
        let query_str = std::include_str!("sql/get_children.sql");
        let children = sqlx::query(query_str)
            .bind(id.to_string())
            .fetch_all(&mut self.pool.acquire().await?)
            .await?;

        let children = children
            .into_iter()
            .flat_map(|row| Uuid::try_parse(row.get(0)))
            .collect();

        Ok(children)
    }

    pub async fn get_roots(&self) -> anyhow::Result<Vec<NodeID>> {
        let query_str = std::include_str!("sql/get_roots.sql");
        let roots = sqlx::query(query_str)
            .fetch_all(&mut self.pool.acquire().await?)
            .await?;

        let roots = roots
            .into_iter()
            .flat_map(|row| Uuid::try_parse(row.get(0)))
            .collect();

        Ok(roots)
    }

    pub async fn dfs(&self, root: NodeID) -> anyhow::Result<DFSIter<'_>> {
        self.exists_check(&root).await?;

        Ok(DFSIter {
            database: self,
            seen: BTreeSet::default(),
            stack: vec![(root, 0)],
        })
    }

    async fn exists_check(&self, id: &NodeID) -> anyhow::Result<()> {
        let nodes = sqlx::query("SELECT * FROM nodes WHERE uuid = ?")
            .bind(id.to_string())
            .fetch_all(&mut self.pool.acquire().await?)
            .await?;
        if nodes.len() == 0 {
            return Err(anyhow!("Missing node {}", id));
        }
        Ok(())
    }
}

pub struct DFSIter<'a> {
    database: &'a Database,
    seen: BTreeSet<NodeID>,
    stack: Vec<(NodeID, usize)>,
}

impl<'a> DFSIter<'a> {
    pub async fn next(&mut self) -> anyhow::Result<Option<(NodeID, usize)>> {
        let Some((next, depth)) = self.stack.pop() else {
	    return Ok(None);
	};
        self.seen.insert(next.clone());
        for child in self.database.get_children(next).await? {
            if self.seen.contains(&child) {
                continue;
            }
            self.stack.push((child, depth + 1));
        }
        Ok(Some((next, depth)))
    }
}

pub type NodeID = Uuid;

#[derive(Deserialize, Serialize)]
pub struct Node {
    pub id: NodeID,
    pub title: String,
    pub description: String,
    #[serde(with = "ts_seconds_option")]
    pub scheduled: Option<DateTime<Utc>>,
    #[serde(with = "ts_seconds_option")]
    pub due: Option<DateTime<Utc>>,
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
        format!("{} ({})", self.title, self.id)
    }
}

impl TryFrom<SqliteRow> for Node {
    type Error = anyhow::Error;

    fn try_from(value: SqliteRow) -> Result<Self, Self::Error> {
        Ok(Self {
            id: Uuid::try_parse(value.get("uuid"))?,
            title: value.get("title"),
            description: value.get("description"),
            scheduled: date_time_from_timestamp(value.get("scheduled"))?,
            due: date_time_from_timestamp(value.get("due"))?,
        })
    }
}

fn date_time_from_timestamp(timestamp: Option<i64>) -> anyhow::Result<Option<DateTime<Utc>>> {
    let Some(timestamp) = timestamp else { return Ok(None); };
    match Utc.timestamp_millis_opt(timestamp) {
        LocalResult::Single(dt) => Ok(Some(dt)),
        x => Err(anyhow!("Couldn't parse DateTime from timestamp: {:?}", x)),
    }
}
