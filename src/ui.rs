use std::time::Duration;

use crossterm::event;
use crossterm::event::Event;
use crossterm::event::KeyCode;
use crossterm::event::KeyModifiers;
use crossterm::terminal::disable_raw_mode;
use crossterm::terminal::enable_raw_mode;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::Constraint;
use ratatui::layout::Direction;
use ratatui::layout::Layout;
use ratatui::widgets;
use ratatui::Terminal;

use crate::db;

// TODO: this code is really bad as-is
// because i've been optimizing for iteration
// and not thinking about structuring it "well."
// figure out how to make it not suck :)

pub async fn main(database: db::Database) -> anyhow::Result<()> {
    // Magic spell to clear the screen with an ANSI escape code.
    print!("{}[2J]", 27 as char);

    let stdout = std::io::stdout();
    let _raw_mode = RawModeGuard::new()?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut node_state = NodeState::new(&database).await?;
    let mut list_state = widgets::ListState::default();
    list_state.select(Some(0));

    loop {
        let selected = list_state.selected();

        terminal.draw(|f| {
            let size = f.size();

            let parts = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
                .split(size);

            let items: Vec<widgets::ListItem> = node_state
                .children
                .iter()
                .map(|node| widgets::ListItem::new(node.title.as_str()))
                .collect();
            let list = widgets::List::new(items)
                .block(
                    widgets::Block::default()
                        .title(node_state.title())
                        .borders(widgets::Borders::ALL),
                )
                .highlight_symbol(">>");
            f.render_stateful_widget(list, parts[0], &mut list_state);

            let (title, body) = match selected {
                None => ("N/A".to_string(), "No node selected".to_string()),
                Some(selected) => {
                    let node = &node_state.children[selected];
                    (node.title.clone(), node.description.clone())
                }
            };
            let paragraph = widgets::Paragraph::new(body)
                .wrap(widgets::Wrap { trim: false })
                .block(
                    widgets::Block::default()
                        .title(format!(" [[ {} ]] ", title))
                        .borders(widgets::Borders::ALL),
                );

            f.render_widget(paragraph, parts[1]);
        })?;

        if !event::poll(Duration::from_millis(1000))? {
            continue;
        }
        let evt = match event::read()? {
            Event::Key(evt) => evt,
            _ => continue,
        };

        if evt.code == KeyCode::Char('c') && evt.modifiers.contains(KeyModifiers::CONTROL) {
            break;
        }

        let selected = match selected {
            None => 0,
            Some(selected) => selected,
        };

        if node_state.children.len() > 0 {
            if evt.code == KeyCode::Up && selected > 0 {
                list_state.select(Some(selected - 1));
            }
            if evt.code == KeyCode::Down && selected < node_state.children.len() - 1 {
                list_state.select(Some(selected + 1));
            }
            if evt.code == KeyCode::Right {
                let selected_node = node_state.children[selected].clone();
                node_state
                    .set_current_node(&database, Some(selected_node))
                    .await?;
                if node_state.children.len() > 0 {
                    list_state.select(Some(0));
                } else {
                    list_state.select(None);
                }
            }
        }
        if evt.code == KeyCode::Left && node_state.current_node.is_some() {
            let new_node = if node_state.parents.len() > 0 {
                Some(node_state.parents[0].clone())
            } else {
                None
            };

            node_state.set_current_node(&database, new_node).await?;
        }
    }

    Ok(())
}

struct RawModeGuard {}

impl RawModeGuard {
    pub fn new() -> anyhow::Result<Self> {
        enable_raw_mode()?;
        Ok(RawModeGuard {})
    }
}

impl Drop for RawModeGuard {
    fn drop(&mut self) {
        disable_raw_mode().expect("Failed to disable raw mode");
    }
}

struct NodeState {
    current_node: Option<db::Node>,
    parents: Vec<db::Node>,
    children: Vec<db::Node>,
}

impl NodeState {
    async fn new(database: &db::Database) -> anyhow::Result<Self> {
        let mut node_state = NodeState {
            current_node: None,
            parents: vec![],
            children: vec![],
        };
        node_state.set_current_node(database, None).await?;
        Ok(node_state)
    }

    async fn set_current_node(
        &mut self,
        database: &db::Database,
        node: Option<db::Node>,
    ) -> anyhow::Result<()> {
        let Some(node) = node else {
	    let root_ids = database.get_roots().await?;
	    let mut roots = Vec::with_capacity(root_ids.len());
	    for root in root_ids.into_iter() {
		roots.push(database.get_node(root).await?);
	    }

	    self.current_node = None;
	    self.parents = vec![];
	    self.children = roots;
	    return Ok(())
	};

        let parent_ids = database.get_parents(node.id).await?;
        let mut parents = Vec::new();
        for parent_id in parent_ids.into_iter() {
            parents.push(database.get_node(parent_id).await?);
        }

        let child_ids = database.get_children(node.id).await?;
        let mut children = Vec::new();
        for child_id in child_ids.into_iter() {
            children.push(database.get_node(child_id).await?);
        }

        self.current_node = Some(node);
        self.parents = parents;
        self.children = children;
        Ok(())
    }

    fn title(&self) -> &str {
        if let Some(current_node) = &self.current_node {
            &current_node.title
        } else {
            "Root"
        }
    }
}
