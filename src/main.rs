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
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Authenticate with a Micropub site
    Auth {
        /// Domain to authenticate with
        domain: String,
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
    List,
    /// Show a draft's content
    Show {
        /// Draft ID to show
        draft_id: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Auth { domain } => {
            micropub::auth::cmd_auth(&domain).await?;
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
            DraftCommands::List => {
                micropub::draft::cmd_list()?;
                Ok(())
            }
            DraftCommands::Show { draft_id } => {
                micropub::draft::cmd_show(&draft_id)?;
                Ok(())
            }
        }
        Commands::Publish { draft } => {
            micropub::publish::cmd_publish(&draft, None).await?;
            Ok(())
        }
        Commands::Backdate { draft, date } => {
            use chrono::DateTime;
            let parsed_date = DateTime::parse_from_rfc3339(&date)
                .context("Invalid date format. Use ISO 8601 (e.g., 2024-01-15T10:30:00Z)")?
                .with_timezone(&chrono::Utc);
            micropub::publish::cmd_publish(&draft, Some(parsed_date)).await?;
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
    }
}
