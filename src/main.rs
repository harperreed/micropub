// ABOUTME: Main entry point for micropub CLI
// ABOUTME: Parses commands and dispatches to appropriate handlers

use anyhow::Context;
use clap::{Parser, Subcommand};
use micropub::Result;

#[derive(Parser)]
#[command(name = "micropub")]
#[command(about = "Ultra-compliant Micropub CLI", long_about = None)]
struct Cli {
    /// Enable verbose output
    #[arg(short, long, global = true)]
    verbose: bool,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Authenticate with a Micropub site
    Auth {
        /// Domain to authenticate with
        domain: String,
        /// OAuth scope (default: "create update delete media")
        #[arg(long)]
        scope: Option<String>,
    },
    /// Draft management commands
    #[command(subcommand)]
    Draft(DraftCommands),
    /// Publish a draft
    Publish {
        /// Path to draft file
        draft: String,
    },
    /// Publish a backdated post
    Backdate {
        /// Path to draft file
        draft: String,
        /// Date to publish (ISO 8601 format)
        #[arg(long)]
        date: String,
    },
    /// Update an existing post
    Update {
        /// URL of post to update
        url: String,
    },
    /// Delete a post
    Delete {
        /// URL of post to delete
        url: String,
    },
    /// Undelete a post
    Undelete {
        /// URL of post to undelete
        url: String,
    },
    /// Debug connection to a profile
    Debug {
        /// Profile name to debug
        profile: String,
    },
    /// Show current authenticated user
    Whoami,
    /// List published posts
    Posts {
        /// Number of posts to show (default: 10)
        #[arg(short, long, default_value = "10")]
        limit: usize,
        /// Offset for pagination (default: 0)
        #[arg(short, long, default_value = "0")]
        offset: usize,
    },
    /// List uploaded media files
    Media {
        /// Number of media items to show (default: 20)
        #[arg(short, long, default_value = "20")]
        limit: usize,
        /// Offset for pagination (default: 0)
        #[arg(short, long, default_value = "0")]
        offset: usize,
    },
    /// Launch interactive TUI (Terminal User Interface)
    Tui,
    /// Start MCP server (Model Context Protocol)
    Mcp,
}

#[derive(Subcommand)]
enum DraftCommands {
    /// Create a new draft
    New,
    /// Edit an existing draft
    Edit {
        /// Draft ID to edit
        draft_id: String,
    },
    /// List all drafts
    List {
        /// Filter by category
        #[arg(long)]
        category: Option<String>,
        /// Number of drafts to show per page (default: 10)
        #[arg(short, long, default_value = "10")]
        limit: usize,
        /// Offset for pagination (default: 0)
        #[arg(short, long, default_value = "0")]
        offset: usize,
    },
    /// Show a draft's content
    Show {
        /// Draft ID to show
        draft_id: String,
    },
    /// Search drafts by content or metadata
    Search {
        /// Search query
        query: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // If no command provided, show help
    if cli.command.is_none() {
        let config = micropub::config::Config::load()?;

        println!(
            r#"
  ███╗   ███╗██╗ ██████╗██████╗  ██████╗ ██████╗ ██╗   ██╗██████╗
  ████╗ ████║██║██╔════╝██╔══██╗██╔═══██╗██╔══██╗██║   ██║██╔══██╗
  ██╔████╔██║██║██║     ██████╔╝██║   ██║██████╔╝██║   ██║██████╔╝
  ██║╚██╔╝██║██║██║     ██╔══██╗██║   ██║██╔═══╝ ██║   ██║██╔══██╗
  ██║ ╚═╝ ██║██║╚██████╗██║  ██║╚██████╔╝██║     ╚██████╔╝██████╔╝
  ╚═╝     ╚═╝╚═╝ ╚═════╝╚═╝  ╚═╝ ╚═════╝ ╚═╝      ╚═════╝ ╚═════╝

              Ultra-compliant Micropub CLI for IndieWeb
"#
        );

        if !config.default_profile.is_empty() {
            println!("  🔐 Authenticated as: {}", config.default_profile);
            println!("\n  Quick commands:");
            println!("    micropub tui              Launch interactive TUI");
            println!("    micropub draft new        Create a new draft");
            println!("    micropub posts            List published posts");
            println!("    micropub whoami           Show current profile");
        } else {
            println!("  To get started, authenticate with your site:");
            println!("    micropub auth <your-domain.com>");
        }

        println!("\n  For more help, run:");
        println!("    micropub --help\n");
        return Ok(());
    }

    match cli.command.unwrap() {
        Commands::Auth { domain, scope } => {
            micropub::auth::cmd_auth(&domain, scope.as_deref()).await?;
            Ok(())
        }
        Commands::Draft(cmd) => match cmd {
            DraftCommands::New => {
                micropub::draft::cmd_new()?;
                Ok(())
            }
            DraftCommands::Edit { draft_id } => {
                micropub::draft::cmd_edit(&draft_id)?;
                Ok(())
            }
            DraftCommands::List {
                category,
                limit,
                offset,
            } => {
                micropub::draft::cmd_list(category.as_deref(), limit, offset)?;
                Ok(())
            }
            DraftCommands::Show { draft_id } => {
                micropub::draft::cmd_show(&draft_id)?;
                Ok(())
            }
            DraftCommands::Search { query } => {
                micropub::draft::cmd_search(&query)?;
                Ok(())
            }
        },
        Commands::Publish { draft } => {
            let _ = micropub::publish::cmd_publish(&draft, None).await?;
            Ok(())
        }
        Commands::Backdate { draft, date } => {
            use chrono::DateTime;
            let parsed_date = DateTime::parse_from_rfc3339(&date)
                .context("Invalid date format. Use ISO 8601 (e.g., 2024-01-15T10:30:00Z)")?
                .with_timezone(&chrono::Utc);
            let _ = micropub::publish::cmd_publish(&draft, Some(parsed_date)).await?;
            Ok(())
        }
        Commands::Update { url } => {
            micropub::operations::cmd_update(&url).await?;
            Ok(())
        }
        Commands::Delete { url } => {
            micropub::operations::cmd_delete(&url).await?;
            Ok(())
        }
        Commands::Undelete { url } => {
            micropub::operations::cmd_undelete(&url).await?;
            Ok(())
        }
        Commands::Debug { profile } => {
            println!("Debug command: {}", profile);
            Ok(())
        }
        Commands::Whoami => {
            micropub::operations::cmd_whoami().await?;
            Ok(())
        }
        Commands::Posts { limit, offset } => {
            micropub::operations::cmd_list_posts(limit, offset).await?;
            Ok(())
        }
        Commands::Media { limit, offset } => {
            micropub::operations::cmd_list_media(limit, offset).await?;
            Ok(())
        }
        Commands::Tui => {
            micropub::tui::run().await?;
            Ok(())
        }
        Commands::Mcp => {
            micropub::mcp::run_server().await?;
            Ok(())
        }
    }
}
