//! Theme system for UI styling
//!
//! Provides consistent styling across all UI components with support
//! for multiple themes and easy customization.

use ratatui::style::{Color, Modifier, Style};

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

    /// Default theme (VS Code dark theme to match demo)
    pub fn default_theme() -> Self {
        Self {
            name: "default".to_string(),
            colors: ColorScheme {
                background: Color::Rgb(30, 30, 30),      // #1e1e1e - main background
                foreground: Color::Rgb(212, 212, 212),   // #d4d4d4 - main text
                primary: Color::Rgb(0, 122, 204),        // #007acc - VS Code blue
                secondary: Color::Rgb(37, 37, 38),       // #252526 - sidebar background
                accent: Color::Rgb(255, 193, 7),         // #ffc107 - yellow accent
                success: Color::Rgb(40, 167, 69),        // #28a745 - green
                warning: Color::Rgb(255, 193, 7),        // #ffc107 - yellow
                error: Color::Rgb(220, 53, 69),          // #dc3545 - red
                info: Color::Rgb(0, 122, 204),           // #007acc - blue
                muted: Color::Rgb(150, 150, 150),        // #969696 - muted text
            },
            styles: StyleScheme::default(),
        }
    }

    /// Dark theme matching VS Code dark theme from tui-demo.html
    pub fn dark_theme() -> Self {
        Self {
            name: "dark".to_string(),
            colors: ColorScheme {
                background: Color::Rgb(30, 30, 30),      // #1e1e1e - main background
                foreground: Color::Rgb(212, 212, 212),   // #d4d4d4 - main text
                primary: Color::Rgb(0, 122, 204),        // #007acc - VS Code blue
                secondary: Color::Rgb(37, 37, 38),       // #252526 - sidebar background
                accent: Color::Rgb(255, 193, 7),         // #ffc107 - yellow accent
                success: Color::Rgb(40, 167, 69),        // #28a745 - green
                warning: Color::Rgb(255, 193, 7),        // #ffc107 - yellow
                error: Color::Rgb(220, 53, 69),          // #dc3545 - red
                info: Color::Rgb(0, 122, 204),           // #007acc - blue
                muted: Color::Rgb(150, 150, 150),        // #969696 - muted text
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

    /// Get style for borders (VS Code style)
    pub fn border_style(&self) -> Style {
        Style::default().fg(Color::Rgb(62, 62, 66)) // #3e3e42 - VS Code border color
    }

    /// Get background style for sidebar
    pub fn sidebar_background_style(&self) -> Style {
        Style::default().bg(self.colors.secondary) // #252526 - sidebar background
    }

    /// Get background style for main content area
    pub fn content_background_style(&self) -> Style {
        Style::default().bg(self.colors.background) // #1e1e1e - main background
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

    /// Get style for status bar/sync status
    pub fn status_style(&self) -> Style {
        Style::default().fg(self.colors.secondary)
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
