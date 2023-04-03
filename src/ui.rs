use std::time::Duration;

use crossterm::event;
use crossterm::event::Event;
use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyModifiers;
use crossterm::terminal::disable_raw_mode;
use crossterm::terminal::enable_raw_mode;
use ratatui::backend::Backend;
use ratatui::backend::CrosstermBackend;
use ratatui::layout;
use ratatui::layout::Constraint;
use ratatui::layout::Direction;
use ratatui::layout::Layout;
use ratatui::widgets;
use ratatui::Frame;
use ratatui::Terminal;

use crate::db;
use crate::find::find_candidates;

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
//     - up = select up
//     - down = select down
//     - enter = choose currently selected node
//     - everything else = normal text editing!
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

    let mut mode = Mode::Normal(NormalState::new(&database, None).await?);
    loop {
        terminal.draw(|f| {
            mode.render(f);
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

        mode = mode.handle_input(&database, evt).await?;
    }

    Ok(())
}

enum Mode {
    Normal(NormalState),
    Find(FindState),
}

impl Mode {
    async fn handle_input(self, database: &db::Database, evt: KeyEvent) -> anyhow::Result<Mode> {
        use Mode::*;
        match self {
            Normal(state) => state.handle_input(database, evt).await,
            Find(state) => state.handle_input(database, evt).await,
        }
    }

    fn render(&mut self, f: &mut Frame<impl Backend>) {
        use Mode::*;
        match self {
            Normal(state) => state.render(f),
            Find(state) => state.render(f),
        }
    }
}

struct NormalState {
    mode: NormalStateMode,
    node_path: Vec<db::Node>,
    current_node: Option<db::Node>,
    children: Vec<db::Node>,
    node_list_state: widgets::ListState,
}

#[derive(Clone, Copy)]
enum NormalStateMode {
    List,
    Title,
    Description,
}

impl NormalStateMode {
    fn next(self) -> Self {
        use NormalStateMode::*;
        match self {
            List => Title,
            Title => Description,
            Description => List,
        }
    }

    fn last(self) -> Self {
        use NormalStateMode::*;
        match self {
            List => Description,
            Title => List,
            Description => Title,
        }
    }
}

impl NormalState {
    async fn new(database: &db::Database, root: Option<db::Node>) -> anyhow::Result<Self> {
        let mut state = Self {
            mode: NormalStateMode::List,
            node_path: vec![],
            current_node: None,
            node_list_state: widgets::ListState::default(),
            children: vec![],
        };
        state.choose_node(database, root).await?;
        Ok(state)
    }

    async fn handle_input(
        mut self,
        database: &db::Database,
        evt: KeyEvent,
    ) -> anyhow::Result<Mode> {
        // TODO: make this also persist state when you exit the program
        if evt.code == KeyCode::BackTab {
            self.mode = self.mode.last();
            self.persist_changes(database).await?;
            return Ok(Mode::Normal(self));
        }
        if evt.code == KeyCode::Tab {
            self.mode = self.mode.next();
            self.persist_changes(database).await?;
            return Ok(Mode::Normal(self));
        }

        use NormalStateMode::*;
        match self.mode {
            List => return self.handle_list_input(database, evt).await,
            Title => self.handle_title_input(evt),
            Description => self.handle_description_input(evt),
        }
        Ok(Mode::Normal(self))
    }

    async fn persist_changes(&self, database: &db::Database) -> anyhow::Result<()> {
        let Some(selected) = self.node_list_state.selected() else {
	    return Ok(());
	};
        let node = &self.children[selected];
        database.update(node).await
    }

    async fn handle_list_input(
        mut self,
        database: &db::Database,
        evt: KeyEvent,
    ) -> anyhow::Result<Mode> {
        if evt.code == KeyCode::Char('f') {
            return Ok(Mode::Find(FindState::new(database, self).await?));
        }

        if evt.code == KeyCode::Up {
            self.go_up();
        }
        if evt.code == KeyCode::Down {
            self.go_down();
        }
        if evt.code == KeyCode::Left {
            self.choose_parent(&database).await?;
        }
        if evt.code == KeyCode::Right {
            self.choose_current_child(&database).await?;
        }
        Ok(Mode::Normal(self))
    }

    fn handle_title_input(&mut self, evt: KeyEvent) {
        let Some(selected) = self.node_list_state.selected() else { return };
        let selected_node = &mut self.children[selected];

        match evt.code {
            KeyCode::Char(c) => {
                selected_node.title.push(c);
            }
            KeyCode::Backspace => {
                selected_node.title.pop();
            }
            _ => {}
        }
    }

    fn handle_description_input(&mut self, evt: KeyEvent) {
        let Some(selected) = self.node_list_state.selected() else { return };
        let selected_node = &mut self.children[selected];

        match evt.code {
            KeyCode::Char(c) => {
                selected_node.description.push(c);
            }
            KeyCode::Enter => {
                selected_node.description.push('\n');
            }
            KeyCode::Backspace => {
                selected_node.description.pop();
            }
            _ => {}
        }
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

        // TODO: fix some things with this:
        // - replace `.len()`s with something that represents the rune-length of a line
        // - handle wrapping
        //   - make the `x` coordinate fit horizontally along the rune-length of the last section of a wrapped line
        //   - make the `y` coordinate account for the vertical length of wrapped lines
        let Some(selected) = self.node_list_state.selected() else { return; };
        use NormalStateMode::*;
        match self.mode {
            List => {}
            Title => {
                let x = parts[1].x
                    + ("- [[ ".len() as u16)
                    + (self.children[selected].title.len() as u16);
                let y = 0;
                f.set_cursor(x, y);
            }
            Description => {
                let lines: Vec<&str> = self.children[selected].description.lines().collect();
                let mut x = parts[1].x + 1;
                if lines.len() > 0 {
                    x += lines[lines.len() - 1].len() as u16;
                };
                let mut y = lines.len() as u16;
                if y == 0 {
                    y += 1;
                }
                f.set_cursor(x, y);
            }
        }
    }

    fn title(&self) -> String {
        match &self.current_node {
            None => "Root".to_string(),
            Some(node) => node.title.clone(),
        }
    }
}

struct FindState {
    parent: NormalState,
    search_string: String,
    candidates: Vec<db::Node>,
    candidate_list_state: widgets::ListState,
}

impl FindState {
    async fn new(database: &db::Database, parent: NormalState) -> anyhow::Result<Self> {
        let mut find_state = FindState {
            parent,
            search_string: "".to_string(),
            candidates: vec![],
            candidate_list_state: widgets::ListState::default(),
        };
        find_state.update_search_candidates(database).await?;
        Ok(find_state)
    }

    async fn handle_input(
        mut self,
        database: &db::Database,
        evt: KeyEvent,
    ) -> anyhow::Result<Mode> {
        let is_ctrl_g =
            evt.modifiers.contains(KeyModifiers::CONTROL) && evt.code == KeyCode::Char('g');
        if evt.code == KeyCode::Esc || is_ctrl_g {
            return Ok(Mode::Normal(self.parent));
        }

        let mut string_changed = false;
        match evt.code {
            KeyCode::Up => self.go_up(),
            KeyCode::Down => self.go_down(),

            KeyCode::Enter => return self.choose(database).await,

            KeyCode::Char(c) => {
                self.search_string.push(c);
                string_changed = true;
            }
            KeyCode::Backspace => {
                self.search_string.pop();
                string_changed = true;
            }

            _ => {}
        }

        if string_changed {
            self.update_search_candidates(database).await?;
        }

        Ok(Mode::Find(self))
    }

    async fn choose(self, database: &db::Database) -> anyhow::Result<Mode> {
        let Some(selected) = self.candidate_list_state.selected() else { return Ok(Mode::Find(self)) };
        Ok(Mode::Normal(
            NormalState::new(database, Some(self.candidates[selected].clone())).await?,
        ))
    }

    fn go_up(&mut self) {
        let Some(selected) = self.candidate_list_state.selected() else { return };
        if selected > 0 {
            self.candidate_list_state.select(Some(selected - 1));
        }
    }

    fn go_down(&mut self) {
        let Some(selected) = self.candidate_list_state.selected() else { return };
        if selected < self.candidates.len() - 1 {
            self.candidate_list_state.select(Some(selected + 1));
        }
    }

    async fn update_search_candidates(&mut self, database: &db::Database) -> anyhow::Result<()> {
        let mut candidates = find_candidates(&self.search_string, database).await?;
        candidates.sort_by_key(|(_, distance)| distance.clone());

        self.candidates = candidates.into_iter().map(|(node, _)| node).collect();
        if self.candidates.len() == 0 {
            self.candidate_list_state.select(None);
        } else {
            self.candidate_list_state.select(Some(0));
        }
        Ok(())
    }

    fn render(&mut self, f: &mut Frame<impl Backend>) {
        self.parent.render(f);

        let rect = {
            let margin = layout::Margin {
                horizontal: 8,
                vertical: 4,
            };
            f.size().inner(&margin)
        };
        f.render_widget(widgets::Clear, rect);

        let parts = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(3), Constraint::Percentage(100)])
            .split(rect);

        // TODO: handle rendering search strings which are longer than the width of this block
        let top = widgets::Paragraph::new(self.search_string.clone())
            .block(widgets::Block::default().borders(widgets::Borders::all()));
        f.render_widget(top, parts[0]);
        f.set_cursor(
            parts[0].x + 1 + self.search_string.len() as u16,
            parts[0].y + 1,
        );

        let bottom = widgets::List::new(
            self.candidates
                .iter()
                .map(|node| widgets::ListItem::new(node.title.to_owned()))
                .collect::<Vec<widgets::ListItem>>(),
        )
        .block(widgets::Block::default().borders(widgets::Borders::all()))
        .highlight_symbol(">>");
        f.render_stateful_widget(bottom, parts[1], &mut self.candidate_list_state);
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
