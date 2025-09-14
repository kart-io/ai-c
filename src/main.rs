use ai_c::{initialize_logging, App, config::Config, error::AppResult, git::GitService};
use std::{env, time::{Duration, Instant}};
use tracing::{info, warn, debug};

#[tokio::main]
async fn main() -> AppResult<()> {
    // Initialize logging first
    initialize_logging().map_err(|e| ai_c::error::AppError::application(&e.to_string()))?;
    let start_time = Instant::now();

    // Check if we should run in demo mode or TUI mode
    let args: Vec<String> = env::args().collect();
    let demo_mode = args.contains(&"--demo".to_string()) ||
                   env::var("AI_C_DEMO_MODE").is_ok() ||
                   env::var("TERM").unwrap_or_default().is_empty();

    if demo_mode {
        info!("🚀 AI-C TUI Demo Mode Starting...");
        run_demo_mode(start_time).await
    } else {
        info!("🚀 AI-C TUI Starting...");
        run_full_tui_mode(start_time).await
    }
}

async fn run_demo_mode(start_time: Instant) -> AppResult<()> {
    // Load configuration
    let config = Config::default();
    info!("📋 Configuration loaded: {}", config.app.name);

    // Initialize Git service (with mock support)
    match GitService::new(&config.git).await {
        Ok(git_service) => {
            info!("✅ Git service initialized successfully");

            // Get repository status
            match git_service.get_status().await {
                Ok(status) => {
                    if status.is_empty() {
                        info!("📁 Repository status: No changes detected (or mock mode)");
                    } else {
                        info!("📁 Repository status: {} files with changes", status.len());
                        for (i, file) in status.iter().take(5).enumerate() {
                            info!("  {}. {} - {}", i + 1, file.path, file.status.status_char());
                        }
                        if status.len() > 5 {
                            info!("  ... and {} more files", status.len() - 5);
                        }
                    }
                }
                Err(e) => {
                    warn!("⚠️  Failed to get repository status: {}", e);
                }
            }
        }
        Err(e) => {
            warn!("⚠️  Failed to initialize Git service: {}", e);
        }
    }

    // Show demo UI components info
    info!("🎨 UI Components Available:");
    info!("  • Status Tab - File changes and staging");
    info!("  • Branches Tab - Branch management");
    info!("  • Tags Tab - Tag management");
    info!("  • Stash Tab - Stash operations");
    info!("  • Remotes Tab - Remote repositories");
    info!("  • GitFlow Tab - GitFlow operations");

    info!("⌨️  Key Bindings:");
    info!("  • Tab - Switch between tabs");
    info!("  • Arrow Keys / j,k - Navigate items");
    info!("  • Space - Toggle selection");
    info!("  • q / Esc - Quit application");

    // Simulate application running
    info!("✨ AI-C TUI Demo completed successfully!");

    let duration = start_time.elapsed();
    info!("⏱️  Total execution time: {:?}", duration);

    if duration > Duration::from_secs(1) {
        warn!("⚠️  Startup time exceeded 1 second target: {:?}", duration);
    } else {
        info!("🎯 Performance target met: < 1 second");
    }

    info!("👋 AI-C TUI Demo finished.");
    Ok(())
}

async fn run_full_tui_mode(start_time: Instant) -> AppResult<()> {
    // Try to initialize the full TUI application
    match App::new().await {
        Ok(app) => {
            let startup_duration = start_time.elapsed();
            debug!("Application startup time: {:?}", startup_duration);

            if startup_duration > Duration::from_secs(1) {
                warn!("⚠️  Startup time exceeded 1 second: {:?}", startup_duration);
            }

            // Try to run the TUI
            match app.run().await {
                Ok(_) => {
                    info!("AI-C TUI application terminated gracefully");
                    Ok(())
                }
                Err(e) => {
                    warn!("TUI mode failed: {}. Falling back to demo mode.", e);
                    warn!("Use 'ai-c --demo' to run in demo mode explicitly.");
                    // Fall back to demo mode
                    run_demo_mode(start_time).await
                }
            }
        }
        Err(e) => {
            warn!("Failed to initialize TUI: {}. Running in demo mode.", e);
            warn!("This might be because:");
            warn!("  • Not in a Git repository");
            warn!("  • Terminal doesn't support TUI mode");
            warn!("  • Missing required dependencies");
            warn!("Use 'ai-c --demo' to run in demo mode explicitly.");
            run_demo_mode(start_time).await
        }
    }
}
