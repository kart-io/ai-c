//! Remotes tab component

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

pub struct RemotesTabComponent {}

impl RemotesTabComponent {
    pub fn new() -> Self {
        Self {}
    }
}

impl Component for RemotesTabComponent {
    fn render(&mut self, frame: &mut Frame, area: Rect, state: &AppState, theme: &Theme) {
        let paragraph = Paragraph::new("Remotes tab - Coming soon")
            .block(
                Block::default()
                    .title("Remotes")
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
