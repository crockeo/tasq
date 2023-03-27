use std::time::Duration;

use crossterm::event;
use crossterm::event::Event;
use crossterm::event::KeyCode;
use crossterm::event::KeyModifiers;
use crossterm::terminal::disable_raw_mode;
use crossterm::terminal::enable_raw_mode;
use ratatui::backend::Backend;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::Constraint;
use ratatui::layout::Direction;
use ratatui::layout::Layout;
use ratatui::widgets;
use ratatui::Frame;
use ratatui::Terminal;

use crate::db;

// TODO: this code is really bad as-is
// because i've been optimizing for iteration
// and not thinking about structuring it "well."
// figure out how to make it not suck :)

// - modes
//   - normal
//     - nodes on the LHS
//     - block on the right w/ the current contents
//       - title = name of the node
//       - body = paragraph
//     - cycle between node list / title / body to interact with each
//     - keybinds
//       - tab -> cycle
//       - a -> add
//       - f -> find
//       - c -> connect
//       - n -> next
//     - for each sub mode: render normal mode behind them
//   - add
//     - TODO
//   - find
//     - TODO
//   - connect
//     - TODO
//   - next
//     - TODO

pub async fn main(database: db::Database) -> anyhow::Result<()> {
    // Magic spell to clear the screen with an ANSI escape code.
    print!("{}[2J]", 27 as char);

    let stdout = std::io::stdout();
    let _raw_mode = RawModeGuard::new()?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut normal_state = NormalState::new(&database).await?;
    loop {
        terminal.draw(|f| {
            normal_state.render(f);
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

        // TODO: move this into NormalState
        if evt.code == KeyCode::Up {
            normal_state.go_up();
        }
        if evt.code == KeyCode::Down {
            normal_state.go_down();
        }
        if evt.code == KeyCode::Left {
            normal_state.choose_parent(&database).await?;
        }
        if evt.code == KeyCode::Right {
            normal_state.choose_current_child(&database).await?;
        }
    }

    Ok(())
}

struct NormalState {
    node_path: Vec<db::Node>,
    current_node: Option<db::Node>,
    children: Vec<db::Node>,
    node_list_state: widgets::ListState,
}

impl NormalState {
    async fn new(database: &db::Database) -> anyhow::Result<Self> {
        let mut state = Self {
            node_path: vec![],
            current_node: None,
            node_list_state: widgets::ListState::default(),
            children: vec![],
        };
        state.choose_node(database, None).await?;
        Ok(state)
    }

    async fn choose_parent(&mut self, database: &db::Database) -> anyhow::Result<()> {
        let next = self.node_path.pop();
        if let (None, None) = (&self.current_node, &next) {
            return Ok(());
        }
        self.choose_node(database, next).await
    }

    async fn choose_current_child(&mut self, database: &db::Database) -> anyhow::Result<()> {
        if self.children.len() == 0 {
            return Ok(());
        }

        if let Some(current_node) = &self.current_node {
            self.node_path.push(current_node.clone());
        };
        let selected = self.node_list_state.selected().unwrap();
        let node = &self.children[selected];
        self.choose_node(database, Some(node.clone())).await
    }

    fn go_up(&mut self) {
        if self.children.len() == 0 {
            return;
        }

        let selected = self.node_list_state.selected().unwrap();
        if selected > 0 {
            self.node_list_state.select(Some(selected - 1));
        }
    }

    fn go_down(&mut self) {
        if self.children.len() == 0 {
            return;
        }

        let selected = self.node_list_state.selected().unwrap();
        if selected < self.children.len() - 1 {
            self.node_list_state.select(Some(selected + 1));
        }
    }

    async fn choose_node(
        &mut self,
        database: &db::Database,
        node: Option<db::Node>,
    ) -> anyhow::Result<()> {
        let child_ids = match &node {
            None => database.get_roots().await,
            Some(node) => database.get_children(node.id).await,
        }?;
        self.children = {
            let mut children = Vec::new();
            for child_id in child_ids.into_iter() {
                children.push(database.get_node(child_id).await?);
            }
            children
        };
        self.current_node = node;

        if self.children.len() == 0 {
            self.node_list_state.select(None)
        } else {
            self.node_list_state.select(Some(0))
        }

        Ok(())
    }

    fn render(&mut self, f: &mut Frame<impl Backend>) {
        let size = f.size();

        let parts = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
            .split(size);

        let items: Vec<widgets::ListItem> = self
            .children
            .iter()
            .map(|node| widgets::ListItem::new(node.title.as_str()))
            .collect();
        let list = widgets::List::new(items)
            .block(
                widgets::Block::default()
                    .title(self.title())
                    .borders(widgets::Borders::ALL),
            )
            .highlight_symbol(">>");
        f.render_stateful_widget(list, parts[0], &mut self.node_list_state);

        let (title, body) = match self.node_list_state.selected() {
            None => ("N/A".to_string(), "No node selected".to_string()),
            Some(selected) => {
                let node = &self.children[selected];
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
    }

    fn title(&self) -> String {
        match &self.current_node {
            None => "Root".to_string(),
            Some(node) => node.title.clone(),
        }
    }
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
