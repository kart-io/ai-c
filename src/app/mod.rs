//! Application core module
//!
//! Contains the main application logic, state management, and event handling system.
//! Performance requirement: Application initialization < 500ms

pub mod events;
pub mod state;

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    Terminal,
};
use std::time::{Duration, Instant};
use tokio::time::{sleep, timeout};
use tracing::{debug, info, warn};

use crate::{
    config::Config,
    error::{AppError, AppResult},
    git::GitService,
    ui::UI,
};
use events::{AppEvent, EventHandler};
use state::AppState;

/// Main application struct
///
/// Manages the entire application lifecycle including:
/// - Terminal setup and cleanup
/// - Event handling and state management
/// - Git operations and AI agent integration
/// - Performance monitoring
pub struct App {
    /// Application state
    state: AppState,
    /// Event handler for async operations
    event_handler: EventHandler,
    /// Git service for repository operations
    git_service: GitService,
    /// UI renderer
    ui: UI,
    /// Application configuration
    config: Config,
    /// Performance metrics
    startup_time: Duration,
}

impl App {
    /// Create a new application instance
    ///
    /// Performance requirement: < 500ms initialization time
    pub async fn new() -> AppResult<Self> {
        let init_start = Instant::now();

        info!("Initializing AI-Commit TUI application");

        // Load configuration - target: < 50ms
        let config_start = Instant::now();
        let config = Config::load().await?;
        debug!("Configuration loaded in {:?}", config_start.elapsed());

        // Initialize Git service - target: < 100ms
        let git_start = Instant::now();
        let git_service = GitService::new(&config.git).await?;
        debug!("Git service initialized in {:?}", git_start.elapsed());

        // Initialize application state - target: < 50ms
        let state_start = Instant::now();
        let mut state = AppState::new();
        state.set_git_service(git_service.clone());
        debug!(
            "Application state initialized in {:?}",
            state_start.elapsed()
        );

        // Initialize event handler - target: < 50ms
        let event_start = Instant::now();
        let event_handler = EventHandler::new().await?;
        debug!("Event handler initialized in {:?}", event_start.elapsed());

        // Initialize UI - target: < 100ms
        let ui_start = Instant::now();
        let ui = UI::new(&config.ui)?;
        debug!("UI initialized in {:?}", ui_start.elapsed());

        let startup_time = init_start.elapsed();

        // Performance validation
        if startup_time > Duration::from_millis(500) {
            warn!(
                "Application initialization exceeded 500ms target: {:?}",
                startup_time
            );
        } else {
            debug!("Application initialized successfully in {:?}", startup_time);
        }

        Ok(Self {
            state,
            event_handler,
            git_service,
            ui,
            config,
            startup_time,
        })
    }

    /// Run the main application loop
    ///
    /// Sets up the terminal, handles events, and manages the UI rendering loop.
    pub async fn run(mut self) -> AppResult<()> {
        info!("Starting application main loop");

        // Setup terminal
        self.setup_terminal()?;

        let result = self.main_loop().await;

        // Cleanup terminal
        self.cleanup_terminal()?;

        result
    }

    /// Setup terminal for TUI
    fn setup_terminal(&self) -> AppResult<()> {
        enable_raw_mode().map_err(|e| {
            warn!("Failed to enable raw mode: {}. Running in limited mode.", e);
            AppError::Io(e)
        })?;
        let mut stdout = std::io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture).map_err(|e| {
            warn!("Failed to setup terminal: {}. Running in limited mode.", e);
            AppError::Io(e)
        })?;
        Ok(())
    }

    /// Cleanup terminal after TUI
    fn cleanup_terminal(&self) -> AppResult<()> {
        disable_raw_mode()?;
        let mut stdout = std::io::stdout();
        execute!(stdout, LeaveAlternateScreen, DisableMouseCapture)?;
        Ok(())
    }

    /// Main application event loop
    async fn main_loop(&mut self) -> AppResult<()> {
        let backend = CrosstermBackend::new(std::io::stdout());
        let mut terminal = Terminal::new(backend)?;

        info!("Entering main application loop");

        loop {
            // Render UI - target: < 16ms for 60fps
            let render_start = Instant::now();
            terminal.draw(|f| {
                self.ui.render(f, &self.state);
            })?;

            let render_time = render_start.elapsed();
            if render_time > Duration::from_millis(16) {
                debug!("Render time exceeded 16ms target: {:?}", render_time);
            }

            // Handle events with timeout for responsiveness
            if let Ok(has_event) = timeout(Duration::from_millis(100), self.handle_events()).await {
                if !has_event? {
                    continue;
                }
            }

            // Check if application should quit
            if self.state.should_quit() {
                info!("Application quit requested");
                break;
            }

            // Process pending tasks
            self.process_background_tasks().await?;

            // Small delay to prevent busy waiting
            sleep(Duration::from_millis(1)).await;
        }

        Ok(())
    }

    /// Handle input events
    async fn handle_events(&mut self) -> AppResult<bool> {
        if !event::poll(Duration::from_millis(0))? {
            return Ok(false);
        }

        match event::read()? {
            Event::Key(key) => {
                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => {
                        self.state.set_should_quit(true);
                        info!("Quit requested by user");
                    }
                    KeyCode::Char('r') => {
                        info!("Refresh requested");
                        self.refresh_git_status().await?;
                    }
                    _ => {
                        // Forward to UI event handler
                        self.ui.handle_key_event(key, &mut self.state)?;
                    }
                }
            }
            Event::Resize(width, height) => {
                debug!("Terminal resized to {}x{}", width, height);
                self.ui.handle_resize(width, height);
            }
            _ => {}
        }

        Ok(true)
    }

    /// Process background tasks and async operations
    async fn process_background_tasks(&mut self) -> AppResult<()> {
        // Check for completed background tasks
        if let Some(event) = self.event_handler.try_receive_event().await {
            self.handle_app_event(event).await?;
        }

        Ok(())
    }

    /// Handle application events from background tasks
    async fn handle_app_event(&mut self, event: AppEvent) -> AppResult<()> {
        match event {
            AppEvent::GitStatusUpdated(status) => {
                debug!("Git status updated with {} files", status.len());
                self.state.update_git_status(status);
            }
            AppEvent::AgentTaskCompleted { task_id, result } => {
                debug!("Agent task {} completed", task_id);
                self.state.update_agent_result(task_id, result);
            }
            AppEvent::Error(error) => {
                warn!("Background task error: {}", error);
                self.state.add_error(error);
            }
            // Handle all other events with default behavior
            AppEvent::GitOperationCompleted { .. } => {
                debug!("Git operation completed");
            }
            AppEvent::AgentTaskFailed { task_id, error, retry_count } => {
                warn!("Agent task {} failed (retry {}): {}", task_id, retry_count, error);
            }
            AppEvent::McpMessageReceived { .. } => {
                debug!("MCP message received");
            }
            AppEvent::McpProtocolError { error, connection_id } => {
                warn!("MCP protocol error on connection {}: {}", connection_id, error);
            }
            AppEvent::PerformanceWarning { .. } => {
                debug!("Performance warning");
            }
            AppEvent::ConfigurationChanged { .. } => {
                debug!("Configuration changed");
            }
            AppEvent::UIStateChanged { .. } => {
                debug!("UI state changed");
            }
            AppEvent::BackgroundTaskStarted { .. } => {
                debug!("Background task started");
            }
            AppEvent::BackgroundTaskCompleted { .. } => {
                debug!("Background task completed");
            }
            AppEvent::Shutdown => {
                debug!("Shutdown requested");
                self.state.set_should_quit(true);
            }
        }
        Ok(())
    }

    /// Refresh Git repository status
    async fn refresh_git_status(&mut self) -> AppResult<()> {
        let refresh_start = Instant::now();

        info!("Refreshing Git repository status");

        // Get fresh status from Git service
        let status = self.git_service.get_status().await?;

        let refresh_time = refresh_start.elapsed();

        // Performance validation - target: < 200ms
        if refresh_time > Duration::from_millis(200) {
            warn!(
                "Git status refresh exceeded 200ms target: {:?}",
                refresh_time
            );
        } else {
            debug!("Git status refreshed in {:?}", refresh_time);
        }

        // Update application state
        self.state.update_git_status(status);

        Ok(())
    }

    /// Get application startup time for performance monitoring
    pub fn startup_time(&self) -> Duration {
        self.startup_time
    }
}
