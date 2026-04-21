use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::process::ExitCode;

mod commands;

#[derive(Parser)]
#[command(name = "badger", version, about = "The Badger language toolchain")]
struct Cli {
    #[arg(long, short, global = true)]
    verbose: bool,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Type-check a program without producing output.
    Check(CheckArgs),

    /// Build a program to an executable or intermediate artifact.
    Build(BuildArgs),

    /// Build and run a program under the interpreter.
    Run(RunArgs),

    /// Dump the dataflow graph IR for a program.
    Graph(GraphArgs),

    /// Format Badger source files in place.
    Format(FormatArgs),
}

#[derive(clap::Args)]
struct CheckArgs {
    #[arg(value_name = "ENTRY")]
    entry: PathBuf,
}

#[derive(clap::Args)]
struct BuildArgs {
    #[arg(value_name = "ENTRY")]
    entry: PathBuf,

    #[arg(long, short, value_name = "PATH")]
    out: Option<PathBuf>,

    #[arg(long, default_value = "release")]
    profile: String,
}

#[derive(clap::Args)]
struct RunArgs {
    #[arg(value_name = "ENTRY")]
    entry: PathBuf,

    #[arg(last = true)]
    program_args: Vec<String>,
}

#[derive(clap::Args)]
struct GraphArgs {
    #[arg(value_name = "ENTRY")]
    entry: PathBuf,

    #[arg(long, value_enum, default_value_t = GraphFormat::Text)]
    format: GraphFormat,
}

#[derive(Copy, Clone, clap::ValueEnum)]
enum GraphFormat {
    Text,
    Dot,
    Json,
}

#[derive(clap::Args)]
struct FormatArgs {
    #[arg(value_name = "PATHS", required = true)]
    paths: Vec<PathBuf>,

    #[arg(long)]
    check: bool,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let ctx = commands::Context { verbose: cli.verbose };

    let result = match cli.command {
        Command::Check(args) => commands::check::run(&ctx, args),
        Command::Build(args) => commands::build::run(&ctx, args),
        Command::Run(args) => commands::run::run(&ctx, args),
        Command::Graph(args) => commands::graph::run(&ctx, args),
        Command::Format(args) => commands::format::run(&ctx, args),
    };

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("error: {err}");
            ExitCode::FAILURE
        }
    }
}
