//! whip - An AI Agent orchestrator using Claude Code.
//!
//! This is the main binary that launches the TUI application.

use std::time::Duration;

use secrecy::SecretString;
use whip_config::Config;
use whip_config::auth::resolve_token;
use whip_github::{CachedIssues, FetchOptions, GitHubClient, IssueCache, issue_to_task};
use whip_protocol::KanbanBoard;
use whip_tui::{App, RunResult, terminal};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load configuration
    let config = Config::load().await.unwrap_or_else(|e| {
        eprintln!("Warning: failed to load config: {e}");
        Config::default()
    });

    // Load board from GitHub BEFORE terminal setup so errors are visible
    let board = if config.has_repositories() {
        load_github_board(&config).await.unwrap_or_else(|e| {
            eprintln!("Warning: failed to load GitHub issues: {e}");
            KanbanBoard::new()
        })
    } else {
        eprintln!("No repositories configured. Add repositories to ~/.config/whip/config.json5");
        KanbanBoard::new()
    };

    // Install panic hook to restore terminal on panic
    terminal::install_panic_hook();

    // Setup terminal
    let mut terminal = terminal::setup_terminal()?;

    let mut app = App::with_config(board, config.clone());

    // Run the main loop, handling refresh requests
    loop {
        match app.run(&mut terminal).await? {
            RunResult::Quit => break,
            RunResult::RefreshRequested => {
                // Show loading indicator
                terminal.draw(|frame| {
                    app.view(frame);
                    // Draw a loading message at the bottom
                    let area = frame.area();
                    let loading_area = ratatui::layout::Rect {
                        x: 0,
                        y: area.height.saturating_sub(1),
                        width: area.width,
                        height: 1,
                    };
                    let loading = ratatui::widgets::Paragraph::new(" Refreshing GitHub issues...")
                        .style(
                            ratatui::style::Style::default()
                                .fg(ratatui::style::Color::Yellow)
                                .add_modifier(ratatui::style::Modifier::BOLD),
                        );
                    frame.render_widget(loading, loading_area);
                })?;

                // Force refresh from GitHub (bypass cache) using the CURRENT config
                let current_config = app.config();
                let board = refresh_github_board(current_config)
                    .await
                    .unwrap_or_else(|e| {
                        // On error, keep the current board
                        eprintln!("\rRefresh failed: {e}");
                        KanbanBoard::new()
                    });
                app.set_board(board);
            }
        }
    }

    // Always restore terminal
    terminal::restore_terminal(&mut terminal)?;

    Ok(())
}

/// Loads a KanbanBoard from GitHub issues.
///
/// Uses caching with the following strategy:
/// 1. Load from cache immediately (fast startup)
/// 2. If cache is stale or missing, fetch from GitHub
/// 3. Save to cache for next startup
async fn load_github_board(config: &Config) -> anyhow::Result<KanbanBoard> {
    let cache = IssueCache::new()?;
    let mut board = KanbanBoard::new();

    // Determine cache staleness threshold from config
    let max_age = Duration::from_secs(u64::from(config.polling.effective_interval(true)));

    for repo in &config.repositories {
        let owner = repo.owner();
        let repo_name = repo.repo();

        eprint!("Loading issues from {owner}/{repo_name}... ");

        // Try to load from cache first
        if let Ok(Some(cached)) = cache.load(owner, repo_name)
            && !cached.is_older_than(max_age)
        {
            // Cache is fresh, use it
            let count = cached.tasks.len();
            for task in cached.tasks {
                board.add_task(task);
            }
            eprintln!("{count} issues (from cache)");
            continue;
        }

        // Cache is stale or missing, fetch from GitHub
        let token = resolve_token(repo, config.github_token.as_deref()).await;
        let authenticated = token.is_some();
        let token = token.map(SecretString::from);

        match GitHubClient::new(token).await {
            Ok(client) => {
                let options = FetchOptions::default();
                match client.fetch_issues(owner, repo_name, &options).await {
                    Ok(issues) => {
                        let tasks: Vec<_> = issues
                            .iter()
                            .map(|issue| issue_to_task(issue, owner, repo_name))
                            .collect();

                        let count = tasks.len();

                        // Save to cache
                        let cached = CachedIssues::new(tasks.clone(), None);
                        let _ = cache.save(owner, repo_name, &cached);

                        for task in tasks {
                            board.add_task(task);
                        }

                        let auth_note = if authenticated {
                            ""
                        } else {
                            " (unauthenticated)"
                        };
                        eprintln!("{count} issues{auth_note}");
                    }
                    Err(e) => {
                        eprintln!("failed: {e}");
                        // Try to use stale cache as fallback
                        if let Ok(Some(cached)) = cache.load(owner, repo_name) {
                            let count = cached.tasks.len();
                            eprintln!("  Using stale cache: {count} issues");
                            for task in cached.tasks {
                                board.add_task(task);
                            }
                        }
                    }
                }
            }
            Err(e) => {
                eprintln!("failed to create client: {e}");
            }
        }
    }

    Ok(board)
}

/// Force-refreshes issues from GitHub, bypassing the cache.
///
/// Used when the user explicitly requests a refresh (Ctrl+R).
async fn refresh_github_board(config: &Config) -> anyhow::Result<KanbanBoard> {
    let cache = IssueCache::new()?;
    let mut board = KanbanBoard::new();

    for repo in &config.repositories {
        let owner = repo.owner();
        let repo_name = repo.repo();

        // Always fetch from GitHub, ignore cache
        let token = resolve_token(repo, config.github_token.as_deref()).await;
        let token = token.map(SecretString::from);

        match GitHubClient::new(token).await {
            Ok(client) => {
                let options = FetchOptions::default();
                match client.fetch_issues(owner, repo_name, &options).await {
                    Ok(issues) => {
                        let tasks: Vec<_> = issues
                            .iter()
                            .map(|issue| issue_to_task(issue, owner, repo_name))
                            .collect();

                        // Update cache
                        let cached = CachedIssues::new(tasks.clone(), None);
                        let _ = cache.save(owner, repo_name, &cached);

                        for task in tasks {
                            board.add_task(task);
                        }
                    }
                    Err(e) => {
                        // On refresh failure, try stale cache
                        if let Ok(Some(cached)) = cache.load(owner, repo_name) {
                            for task in cached.tasks {
                                board.add_task(task);
                            }
                        } else {
                            return Err(e.into());
                        }
                    }
                }
            }
            Err(e) => {
                return Err(e.into());
            }
        }
    }

    Ok(board)
}
