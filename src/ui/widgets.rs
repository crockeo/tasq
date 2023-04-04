use std::rc::Rc;

use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use ratatui::buffer::Buffer;
use ratatui::layout;
use ratatui::widgets;
use ratatui::widgets::StatefulWidget;
use ratatui::widgets::Widget;

use crate::db;
use crate::ui::util;

pub struct NodeEditor {}

impl Default for NodeEditor {
    fn default() -> Self {
        Self {}
    }
}

impl StatefulWidget for NodeEditor {
    type State = NodeEditorState;

    fn render(self, area: layout::Rect, buf: &mut Buffer, state: &mut NodeEditorState) {
	let parts = state.segment_area(area);
        let (title, description) = match &state.node {
            None => ("N/A".to_string(), "No node selected".to_string()),
            Some(node) => (node.title.clone(), node.description.clone()),
        };

        // TODO: handle rendering strings which are longer than the width of `area`
        let top = widgets::Paragraph::new(title)
            .block(widgets::Block::default().borders(widgets::Borders::all()));
        top.render(parts[0], buf);

        let bottom = widgets::Paragraph::new(description)
            .block(widgets::Block::default().borders(widgets::Borders::all()));
        bottom.render(parts[1], buf);
    }
}

pub struct NodeEditorState {
    node: Option<db::Node>,
    mode: NodeEditorMode,
}

impl NodeEditorState {
    pub fn new(node: Option<db::Node>) -> Self {
        Self {
            node,
            mode: NodeEditorMode::Title,
        }
    }

    pub fn select(&mut self, new_node: Option<db::Node>) {
        self.node = new_node;
        self.mode = NodeEditorMode::Title;
    }

    pub fn node(&self) -> Option<&db::Node> {
	self.node.as_ref()
    }

    pub fn handle_input(&mut self, evt: KeyEvent) {
        let Some(node) = &mut self.node else { return };

        let (target, allow_newline) = match self.mode {
            NodeEditorMode::Title => (&mut node.title, false),
            NodeEditorMode::Description => (&mut node.description, true),
        };
        match evt.code {
            KeyCode::Tab => self.mode = self.mode.next(),
            KeyCode::Char(c) => {
                target.push(c);
            }
            KeyCode::Enter if allow_newline => {
                target.push('\n');
            }
            KeyCode::Backspace => {
                target.pop();
            }
            _ => {}
        }
    }

    pub fn cursor_offset(&self, area: layout::Rect) -> Option<(u16, u16)> {
	let Some(node) = &self.node else { return None };
	let parts = self.segment_area(area);
	let ((x, y), part) = match self.mode {
	    NodeEditorMode::Title => (util::cursor_offset(&node.title), parts[0]),
	    NodeEditorMode::Description => (util::cursor_offset(&node.description), parts[1]),
	};
	Some((
	    // NOTE: NodeEditor adds a margin of 1 to an `area`
	    // because it uses Borders::all().
	    x + part.x,
	    y + part.y + 1,
	))
    }

    fn segment_area(&self, area: layout::Rect) -> Rc<[layout::Rect]> {
        layout::Layout::default()
            .direction(layout::Direction::Vertical)
            .constraints([
                layout::Constraint::Min(3),
                layout::Constraint::Percentage(100),
            ])
            .split(area)
    }
}

#[derive(Clone, Copy)]
enum NodeEditorMode {
    Title,
    Description,
}

impl NodeEditorMode {
    fn next(self) -> Self {
        use NodeEditorMode::*;
        match self {
            Title => Description,
            Description => Title,
        }
    }
}
