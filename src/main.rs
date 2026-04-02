use clap::{CommandFactory, Parser, Subcommand};

use zoom_cli::config::Config;
use zoom_cli::output::{OutputConfig, exit_codes};
use zoom_cli::{api, commands};

#[derive(Parser)]
#[command(
    name = "zoom",
    version,
    about = "CLI for the Zoom API",
    arg_required_else_help = true
)]
struct Cli {
    /// Config profile to use [env: ZOOM_PROFILE]
    #[arg(long, env = "ZOOM_PROFILE", global = true)]
    profile: Option<String>,

    /// Output as JSON (auto-enabled when stdout is not a terminal)
    #[arg(long, global = true)]
    json: bool,

    /// Suppress non-data output (counts, confirmations)
    #[arg(long, global = true)]
    quiet: bool,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Manage meetings
    #[command(subcommand, arg_required_else_help = true)]
    Meetings(MeetingsCommand),

    /// Manage recordings
    #[command(subcommand, arg_required_else_help = true)]
    Recordings(RecordingsCommand),

    /// Manage users
    #[command(subcommand, arg_required_else_help = true)]
    Users(UsersCommand),

    /// Meeting and usage reports
    #[command(subcommand, arg_required_else_help = true)]
    Reports(ReportsCommand),

    /// Print schema/field reference for a resource
    Schema {
        /// Resource name: meetings, recordings, users
        resource: String,
    },

    /// Generate shell completions
    Completions {
        /// Shell to generate completions for
        shell: clap_complete::Shell,
    },
}

#[derive(Subcommand)]
enum MeetingsCommand {
    /// List meetings for a user
    List {
        #[arg(long, default_value = "me")]
        user: String,
        #[arg(long)]
        r#type: Option<String>,
    },
    /// Get a meeting by ID
    Get { id: u64 },
    /// Create a meeting
    Create {
        #[arg(long)]
        topic: String,
        #[arg(long)]
        duration: Option<u32>,
        #[arg(long)]
        start: Option<String>,
        #[arg(long)]
        password: Option<String>,
    },
    /// Update a meeting
    Update {
        id: u64,
        #[arg(long)]
        topic: Option<String>,
        #[arg(long)]
        duration: Option<u32>,
        #[arg(long)]
        start: Option<String>,
    },
    /// Delete a meeting
    Delete { id: u64 },
    /// End a live meeting
    End { id: u64 },
    /// List participants from a past meeting
    Participants {
        /// Meeting ID or UUID
        meeting_id: String,
    },
}

#[derive(Subcommand)]
enum RecordingsCommand {
    /// List cloud recordings for a user
    List {
        #[arg(long, default_value = "me")]
        user: String,
        #[arg(long)]
        from: Option<String>,
        #[arg(long)]
        to: Option<String>,
    },
    /// Get recording details for a meeting
    Get {
        /// Meeting ID or UUID
        meeting_id: String,
    },
    /// Download recording files for a meeting
    Download {
        /// Meeting ID or UUID
        meeting_id: String,
        #[arg(long, default_value = ".")]
        out: String,
    },
    /// Start cloud recording for a live meeting
    Start {
        /// Numeric meeting ID of the live meeting
        meeting_id: u64,
    },
    /// Stop cloud recording for a live meeting
    Stop {
        /// Numeric meeting ID of the live meeting
        meeting_id: u64,
    },
    /// Pause cloud recording for a live meeting
    Pause {
        /// Numeric meeting ID of the live meeting
        meeting_id: u64,
    },
    /// Resume cloud recording for a live meeting
    Resume {
        /// Numeric meeting ID of the live meeting
        meeting_id: u64,
    },
}

#[derive(Subcommand)]
enum UsersCommand {
    /// List users in the account
    List {
        #[arg(long)]
        status: Option<String>,
    },
    /// Get a user by ID or email
    Get { id_or_email: String },
    /// Get the current user
    Me,
}

#[derive(Subcommand)]
enum ReportsCommand {
    /// Meeting summary report for a user
    Meetings {
        #[arg(long, default_value = "me")]
        user: String,
        /// Start date (YYYY-MM-DD)
        #[arg(long)]
        from: String,
        /// End date (YYYY-MM-DD, default: today)
        #[arg(long)]
        to: Option<String>,
    },
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    let out = OutputConfig::new(cli.json, cli.quiet);

    // Schema and completions do not require credentials.
    match &cli.command {
        Command::Schema { resource } => {
            commands::schema(resource, &out);
            return;
        }
        Command::Completions { shell } => {
            clap_complete::generate(*shell, &mut Cli::command(), "zoom", &mut std::io::stdout());
            return;
        }
        _ => {}
    }

    let cfg = match Config::load(cli.profile) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(exit_codes::CONFIG_ERROR);
        }
    };

    let mut client = api::ZoomClient::new(cfg.account_id, cfg.client_id, cfg.client_secret);

    let result = match cli.command {
        Command::Meetings(cmd) => match cmd {
            MeetingsCommand::List { user, r#type } => {
                commands::meetings::list(&mut client, &out, &user, r#type.as_deref()).await
            }
            MeetingsCommand::Get { id } => commands::meetings::get(&mut client, &out, id).await,
            MeetingsCommand::Create {
                topic,
                duration,
                start,
                password,
            } => {
                commands::meetings::create(&mut client, &out, topic, duration, start, password)
                    .await
            }
            MeetingsCommand::Update {
                id,
                topic,
                duration,
                start,
            } => commands::meetings::update(&mut client, &out, id, topic, duration, start).await,
            MeetingsCommand::Delete { id } => {
                commands::meetings::delete(&mut client, &out, id).await
            }
            MeetingsCommand::End { id } => commands::meetings::end(&mut client, &out, id).await,
            MeetingsCommand::Participants { meeting_id } => {
                commands::meetings::participants(&mut client, &out, &meeting_id).await
            }
        },
        Command::Recordings(cmd) => match cmd {
            RecordingsCommand::List { user, from, to } => {
                commands::recordings::list(&mut client, &out, &user, from.as_deref(), to.as_deref())
                    .await
            }
            RecordingsCommand::Get { meeting_id } => {
                commands::recordings::get(&mut client, &out, &meeting_id).await
            }
            RecordingsCommand::Download {
                meeting_id,
                out: out_dir,
            } => commands::recordings::download(&mut client, &out, &meeting_id, &out_dir).await,
            RecordingsCommand::Start { meeting_id } => {
                commands::recordings::control(&mut client, &out, meeting_id, "start").await
            }
            RecordingsCommand::Stop { meeting_id } => {
                commands::recordings::control(&mut client, &out, meeting_id, "stop").await
            }
            RecordingsCommand::Pause { meeting_id } => {
                commands::recordings::control(&mut client, &out, meeting_id, "pause").await
            }
            RecordingsCommand::Resume { meeting_id } => {
                commands::recordings::control(&mut client, &out, meeting_id, "resume").await
            }
        },
        Command::Users(cmd) => match cmd {
            UsersCommand::List { status } => {
                commands::users::list(&mut client, &out, status.as_deref()).await
            }
            UsersCommand::Get { id_or_email } => {
                commands::users::get(&mut client, &out, &id_or_email).await
            }
            UsersCommand::Me => commands::users::me(&mut client, &out).await,
        },
        Command::Reports(cmd) => match cmd {
            ReportsCommand::Meetings { user, from, to } => {
                commands::reports::meetings(&mut client, &out, &user, &from, to.as_deref()).await
            }
        },
        Command::Schema { .. } | Command::Completions { .. } => unreachable!(),
    };

    if let Err(e) = result {
        eprintln!("{e}");
        std::process::exit(exit_codes::for_error(&e));
    }
}
