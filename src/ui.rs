use crossterm::terminal::disable_raw_mode;
use crossterm::terminal::enable_raw_mode;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::Constraint;
use ratatui::layout::Direction;
use ratatui::layout::Layout;
use ratatui::widgets::Block;
use ratatui::widgets::Borders;
use ratatui::widgets::List;
use ratatui::widgets::ListItem;
use ratatui::widgets::ListState;
use ratatui::Terminal;

use crate::db;

pub async fn main(database: db::Database) -> anyhow::Result<()> {
    // Magic spell to clear the screen with an ANSI escape code.
    print!("{}[2J]", 27 as char);

    let stdout = std::io::stdout();
    let _ = RawModeGuard::new();

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let root_ids = database.get_roots().await?;
    let mut roots = Vec::with_capacity(root_ids.len());
    for root in root_ids.into_iter() {
        roots.push(database.get_node(root).await?);
    }

    let mut list_state = ListState::default();
    list_state.select(Some(0));

    terminal.draw(|f| {
        let size = f.size();

        let parts = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(size);

        let items: Vec<ListItem> = roots
            .iter()
            .map(|node| ListItem::new(node.title.as_str()))
            .collect();
        let list = List::new(items)
            .block(Block::default().title("Nodes").borders(Borders::ALL))
            .highlight_symbol(">>");

        let block = Block::default().title("Block").borders(Borders::ALL);

	f.render_stateful_widget(list, parts[0], &mut list_state);
        f.render_widget(block, parts[1]);
    })?;

    std::thread::sleep(std::time::Duration::from_secs(5));

    Ok(())
}

pub struct RawModeGuard {}

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
