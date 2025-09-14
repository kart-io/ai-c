//! Theme system for UI styling
//!
//! Provides consistent styling across all UI components with support
//! for multiple themes and easy customization.

use ratatui::style::{Color, Modifier, Style};
use serde::{Deserialize, Serialize};

use crate::error::{AppError, AppResult};

/// UI theme containing all style definitions
#[derive(Debug, Clone)]
pub struct Theme {
    /// Theme name
    pub name: String,
    /// Color scheme
    pub colors: ColorScheme,
    /// Text styles
    pub styles: StyleScheme,
}

impl Theme {
    /// Load a theme by name
    pub fn load(theme_name: &str) -> AppResult<Self> {
        match theme_name {
            "default" => Ok(Self::default_theme()),
            "dark" => Ok(Self::dark_theme()),
            "light" => Ok(Self::light_theme()),
            _ => {
                // Try to load custom theme
                Self::load_custom_theme(theme_name).or_else(|_| Ok(Self::default_theme()))
            }
        }
    }

    /// Default theme (dark with blue accents)
    pub fn default_theme() -> Self {
        Self {
            name: "default".to_string(),
            colors: ColorScheme {
                background: Color::Reset,
                foreground: Color::White,
                primary: Color::Blue,
                secondary: Color::Cyan,
                accent: Color::Yellow,
                success: Color::Green,
                warning: Color::Yellow,
                error: Color::Red,
                info: Color::Blue,
                muted: Color::DarkGray,
            },
            styles: StyleScheme::default(),
        }
    }

    /// Dark theme with softer colors
    pub fn dark_theme() -> Self {
        Self {
            name: "dark".to_string(),
            colors: ColorScheme {
                background: Color::Black,
                foreground: Color::Rgb(220, 220, 220),
                primary: Color::Rgb(100, 149, 237),
                secondary: Color::Rgb(72, 209, 204),
                accent: Color::Rgb(255, 215, 0),
                success: Color::Rgb(50, 205, 50),
                warning: Color::Rgb(255, 165, 0),
                error: Color::Rgb(220, 20, 60),
                info: Color::Rgb(135, 206, 235),
                muted: Color::Rgb(105, 105, 105),
            },
            styles: StyleScheme::default(),
        }
    }

    /// Light theme for better visibility
    pub fn light_theme() -> Self {
        Self {
            name: "light".to_string(),
            colors: ColorScheme {
                background: Color::White,
                foreground: Color::Black,
                primary: Color::Rgb(0, 100, 200),
                secondary: Color::Rgb(0, 150, 150),
                accent: Color::Rgb(200, 150, 0),
                success: Color::Rgb(0, 150, 0),
                warning: Color::Rgb(200, 100, 0),
                error: Color::Rgb(200, 0, 0),
                info: Color::Rgb(0, 100, 200),
                muted: Color::Rgb(120, 120, 120),
            },
            styles: StyleScheme::default(),
        }
    }

    /// Load custom theme from file
    fn load_custom_theme(_theme_name: &str) -> AppResult<Self> {
        // TODO: Implement custom theme loading from TOML files
        Err(AppError::application(
            "Custom theme loading not implemented",
        ))
    }

    /// Get style for borders
    pub fn border_style(&self) -> Style {
        Style::default().fg(self.colors.muted)
    }

    /// Get style for normal text
    pub fn text_style(&self) -> Style {
        Style::default().fg(self.colors.foreground)
    }

    /// Get style for selected/highlighted text
    pub fn highlight_style(&self) -> Style {
        Style::default()
            .fg(self.colors.background)
            .bg(self.colors.primary)
            .add_modifier(Modifier::BOLD)
    }

    /// Get style for tab headers
    pub fn tab_style(&self) -> Style {
        Style::default().fg(self.colors.foreground)
    }

    /// Get style for selected tab
    pub fn tab_highlight_style(&self) -> Style {
        Style::default()
            .fg(self.colors.accent)
            .add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
    }

    /// Get style for success messages
    pub fn success_style(&self) -> Style {
        Style::default()
            .fg(self.colors.success)
            .add_modifier(Modifier::BOLD)
    }

    /// Get style for warning messages
    pub fn warning_style(&self) -> Style {
        Style::default()
            .fg(self.colors.warning)
            .add_modifier(Modifier::BOLD)
    }

    /// Get style for error messages
    pub fn error_style(&self) -> Style {
        Style::default()
            .fg(self.colors.error)
            .add_modifier(Modifier::BOLD)
    }

    /// Get style for info messages
    pub fn info_style(&self) -> Style {
        Style::default().fg(self.colors.info)
    }

    /// Get style for muted/disabled text
    pub fn muted_style(&self) -> Style {
        Style::default().fg(self.colors.muted)
    }

    /// Get style for Git status indicators
    pub fn git_status_style(&self, status_char: char) -> Style {
        match status_char {
            'M' => Style::default().fg(self.colors.warning), // Modified
            'A' | 'S' => Style::default().fg(self.colors.success), // Added/Staged
            'D' => Style::default().fg(self.colors.error),   // Deleted
            '?' => Style::default().fg(self.colors.muted),   // Untracked
            'C' => Style::default()
                .fg(self.colors.error)
                .add_modifier(Modifier::BOLD), // Conflict
            _ => Style::default().fg(self.colors.foreground),
        }
    }
}

/// Color scheme for themes
#[derive(Debug, Clone)]
pub struct ColorScheme {
    pub background: Color,
    pub foreground: Color,
    pub primary: Color,
    pub secondary: Color,
    pub accent: Color,
    pub success: Color,
    pub warning: Color,
    pub error: Color,
    pub info: Color,
    pub muted: Color,
}

/// Style scheme for text formatting
#[derive(Debug, Clone)]
pub struct StyleScheme {
    pub heading1: Style,
    pub heading2: Style,
    pub heading3: Style,
    pub emphasis: Style,
    pub strong: Style,
    pub code: Style,
    pub link: Style,
}

impl Default for StyleScheme {
    fn default() -> Self {
        Self {
            heading1: Style::default().add_modifier(Modifier::BOLD),
            heading2: Style::default().add_modifier(Modifier::BOLD),
            heading3: Style::default().add_modifier(Modifier::BOLD),
            emphasis: Style::default().add_modifier(Modifier::ITALIC),
            strong: Style::default().add_modifier(Modifier::BOLD),
            code: Style::default().fg(Color::Yellow),
            link: Style::default()
                .fg(Color::Blue)
                .add_modifier(Modifier::UNDERLINED),
        }
    }
}
