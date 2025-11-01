use clap::{Parser, Subcommand};

pub mod ansi;
pub mod napm;

pub mod commands {
    pub mod files;
    pub mod info;
    pub mod install;
    pub mod list;
    pub mod query;
    pub mod remove;
    pub mod search;
    pub mod update;
}

#[derive(Parser)]
#[command(name = "napm")]
#[command(about = "NeoArch Package Manager")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Files {
        package: String,
    },
    Info {
        package: String,
    },
    Install {
        packages: Vec<String>,
    },
    List,
    Query {
        file: String,
        #[arg(long, default_value_t = false)]
        fetch: bool,
    },
    Remove {
        packages: Vec<String>,
        #[arg(long, default_value_t = false)]
        deep: bool,
    },
    Search {
        package: String,
        #[arg(long, short)]
        num_results: Option<u32>,
    },
    Update,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Files { package } => commands::files::run(&package),
        Commands::Info { package } => commands::info::run(&package),
        Commands::Install { packages } => commands::install::run(
            packages
                .iter()
                .map(|s| s.as_str())
                .collect::<Vec<_>>()
                .as_slice(),
        ),
        Commands::List => commands::list::run(),
        Commands::Query { file, fetch } => commands::query::run(&file, fetch),
        Commands::Remove { packages, deep } => commands::remove::run(
            packages
                .iter()
                .map(|s| s.as_str())
                .collect::<Vec<_>>()
                .as_slice(),
            deep,
        ),
        Commands::Search {
            package,
            num_results,
        } => commands::search::run(&package, num_results),
        Commands::Update => commands::update::run(),
    }?;

    Ok(())
}
