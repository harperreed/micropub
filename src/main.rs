// ABOUTME: Main entry point for micropub CLI
// ABOUTME: Parses commands and dispatches to appropriate handlers

use clap::{Parser, Subcommand};
use micropub::Result;

#[derive(Parser)]
#[command(name = "micropub")]
#[command(about = "Ultra-compliant Micropub CLI", long_about = None)]
struct Cli {
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
            println!("Auth command: {}", domain);
            Ok(())
        }
        Commands::Draft(cmd) => match cmd {
            DraftCommands::New => {
                micropub::draft::cmd_new().await?;
                Ok(())
            }
            DraftCommands::Edit { draft_id } => {
                micropub::draft::cmd_edit(&draft_id).await?;
                Ok(())
            }
            DraftCommands::List => {
                micropub::draft::cmd_list().await?;
                Ok(())
            }
            DraftCommands::Show { draft_id } => {
                micropub::draft::cmd_show(&draft_id).await?;
                Ok(())
            }
        }
        Commands::Publish { draft } => {
            println!("Publish command: {}", draft);
            Ok(())
        }
        Commands::Backdate { draft, date } => {
            println!("Backdate command: {} at {}", draft, date);
            Ok(())
        }
        Commands::Update { url } => {
            println!("Update command: {}", url);
            Ok(())
        }
        Commands::Delete { url } => {
            println!("Delete command: {}", url);
            Ok(())
        }
        Commands::Undelete { url } => {
            println!("Undelete command: {}", url);
            Ok(())
        }
        Commands::Debug { profile } => {
            println!("Debug command: {}", profile);
            Ok(())
        }
    }
}
