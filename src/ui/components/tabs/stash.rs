//! Stash tab component

use crate::{
    app::state::AppState,
    error::AppResult,
    ui::{components::Component, theme::Theme},
};
use crossterm::event::KeyEvent;
use ratatui::{
    backend::Backend,
    layout::Rect,
    widgets::{Block, Borders, Paragraph},
    Frame,
};

pub struct StashTabComponent {}

impl StashTabComponent {
    pub fn new() -> Self {
        Self {}
    }
}

impl Component for StashTabComponent {
    fn render(&mut self, frame: &mut Frame, area: Rect, state: &AppState, theme: &Theme) {
        let paragraph = Paragraph::new("Stash tab - Coming soon")
            .block(
                Block::default()
                    .title("Stash")
                    .borders(Borders::ALL)
                    .border_style(theme.border_style()),
            )
            .style(theme.text_style());
        frame.render_widget(paragraph, area);
    }

    fn handle_key_event(&mut self, _key: KeyEvent, _state: &mut AppState) -> AppResult<()> {
        Ok(())
    }
}
