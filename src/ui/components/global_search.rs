//! Global search manager that integrates search and filter components

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    Frame,
};

use crate::{
    app::state::AppState,
    error::AppResult,
    ui::theme::Theme,
};

use super::{Component, SearchComponent, FilterComponent};

/// Global search manager that coordinates search and filter functionality
pub struct GlobalSearchManager {
    search_component: SearchComponent,
    filter_component: FilterComponent,
    is_active: bool,
    active_component: ActiveComponent,
}

#[derive(Debug, Clone, PartialEq)]
enum ActiveComponent {
    Search,
    Filter,
}

impl GlobalSearchManager {
    pub fn new() -> Self {
        Self {
            search_component: SearchComponent::new(),
            filter_component: FilterComponent::new(),
            is_active: false,
            active_component: ActiveComponent::Search,
        }
    }

    /// Activate global search
    pub fn activate_search(&mut self) {
        self.is_active = true;
        self.active_component = ActiveComponent::Search;
        self.search_component.activate();
    }

    /// Activate filter manager
    pub fn activate_filter(&mut self) {
        self.is_active = true;
        self.active_component = ActiveComponent::Filter;
        self.filter_component.open();
    }

    /// Deactivate global search
    pub fn deactivate(&mut self) {
        self.is_active = false;
        self.search_component.deactivate();
        self.filter_component.close();
    }

    /// Check if search is active
    pub fn is_active(&self) -> bool {
        self.is_active
    }

    /// Get search component reference
    pub fn search_component(&self) -> &SearchComponent {
        &self.search_component
    }

    /// Get filter component reference
    pub fn filter_component(&self) -> &FilterComponent {
        &self.filter_component
    }

    /// Get mutable search component reference
    pub fn search_component_mut(&mut self) -> &mut SearchComponent {
        &mut self.search_component
    }

    /// Get mutable filter component reference
    pub fn filter_component_mut(&mut self) -> &mut FilterComponent {
        &mut self.filter_component
    }

    /// Switch between search and filter
    pub fn switch_component(&mut self) {
        match self.active_component {
            ActiveComponent::Search => {
                self.active_component = ActiveComponent::Filter;
                self.search_component.deactivate();
                self.filter_component.open();
            }
            ActiveComponent::Filter => {
                self.active_component = ActiveComponent::Search;
                self.filter_component.close();
                self.search_component.activate();
            }
        }
    }

    /// Apply current filters to search results
    pub fn apply_filters_to_search(&mut self) {
        // TODO: Implement filter application to search results
        // This would involve getting active filters from filter_component
        // and applying them to the search results in search_component
    }

    /// Get global keyboard shortcuts help text
    pub fn get_shortcuts_help(&self) -> Vec<(&'static str, &'static str)> {
        vec![
            ("Ctrl+F", "Global Search"),
            ("Ctrl+Shift+F", "Advanced Filter"),
            ("F3", "Toggle Filter Panel"),
            ("Ctrl+G", "Find Next"),
            ("Ctrl+Shift+G", "Find Previous"),
            ("Escape", "Close Search"),
        ]
    }

    /// Handle global keyboard shortcuts
    pub fn handle_global_shortcut(&mut self, key: KeyEvent) -> AppResult<bool> {
        match key.code {
            KeyCode::Char('f') if key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) => {
                if key.modifiers.contains(crossterm::event::KeyModifiers::SHIFT) {
                    // Ctrl+Shift+F: Advanced Filter
                    self.activate_filter();
                } else {
                    // Ctrl+F: Global Search
                    self.activate_search();
                }
                Ok(true)
            }
            KeyCode::F(3) if self.is_active => {
                // F3: Toggle between search and filter
                self.switch_component();
                Ok(true)
            }
            KeyCode::Char('g') if key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) => {
                // Ctrl+G / Ctrl+Shift+G: Find next/previous
                // TODO: Implement find next/previous functionality
                Ok(true)
            }
            KeyCode::Esc if self.is_active => {
                // Escape: Close search
                self.deactivate();
                Ok(true)
            }
            _ => Ok(false),
        }
    }
}

impl Component for GlobalSearchManager {
    fn render(&mut self, frame: &mut Frame, area: Rect, state: &AppState, theme: &Theme) {
        if !self.is_active {
            return;
        }

        match self.active_component {
            ActiveComponent::Search => {
                // Search takes full area when active
                self.search_component.render(frame, area, state, theme);

                // If filter is also visible, show it as overlay
                if self.filter_component.is_open() {
                    self.filter_component.render(frame, area, state, theme);
                }
            }
            ActiveComponent::Filter => {
                // Filter takes full area when active
                self.filter_component.render(frame, area, state, theme);
            }
        }
    }

    fn handle_key_event(&mut self, key: KeyEvent, state: &mut AppState) -> AppResult<()> {
        if !self.is_active {
            return Ok(());
        }

        // First check for global shortcuts
        if self.handle_global_shortcut(key)? {
            return Ok(());
        }

        // Then delegate to active component
        match self.active_component {
            ActiveComponent::Search => {
                self.search_component.handle_key_event(key, state)?;
            }
            ActiveComponent::Filter => {
                self.filter_component.handle_key_event(key, state)?;
            }
        }

        // Check if components deactivated themselves
        if !self.search_component.is_active() && !self.filter_component.is_open() {
            self.is_active = false;
        }

        Ok(())
    }
}

