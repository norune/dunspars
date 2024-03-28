mod commands;
mod display;
pub mod utils;

use crate::resource::{Config, ConfigBuilder};
use crate::VERSION;
use commands::{
    AbilityCommand, Command, ConfigCommand, CoverageCommand, MatchCommand, MoveCommand,
    PokemonCommand, ResourceCommand, SetupCommand, TypeCommand,
};

use std::io::stdout;

use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(author, version = VERSION, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
    /// Sets the mainline Pokémon game the output will be based on
    #[clap(short, long, global = true)]
    game: Option<String>,
    /// Force output to include colors
    #[clap(long, action = clap::ArgAction::SetTrue, global = true)]
    color: bool,
    /// Force output to exclude colors
    #[clap(long, action = clap::ArgAction::SetTrue, global = true)]
    no_color: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Retrieve and set up program data. Run this before using the program
    Setup,
    /// Prints general data about a Pokémon
    Pokemon {
        /// Name of the Pokémon
        pokemon: String,
        /// Display all move data the Pokémon is capable of learning
        #[arg(short, long, action = clap::ArgAction::SetTrue)]
        moves: bool,
        /// Display the Pokémon evolutionary line
        #[arg(short, long, action = clap::ArgAction::SetTrue)]
        evolution: bool,
    },
    /// Prints matchup data between Pokémon
    Match {
        /// Names of the defending Pokémon; max 6
        #[arg(required = true, num_args = 1..=6)]
        defenders: Vec<String>,
        /// Name of the attacking Pokémon
        attacker: String,
        /// Display only moves that match the user's type
        #[arg(short, long, action = clap::ArgAction::SetTrue)]
        stab_only: bool,
        /// Display verbose output
        #[arg(short, long, action = clap::ArgAction::SetTrue)]
        verbose: bool,
    },
    /// Prints type coverage based on the provided Pokémon
    Coverage {
        /// Names of Pokémon; max 6
        #[arg(required = true, num_args = 1..=6)]
        pokemon: Vec<String>,
    },
    /// Prints type weakness and coverage about a Pokémon type or a combination of two
    Type {
        /// Name of a type
        primary_type: String,
        /// Name of a secondary type. Optional
        secondary_type: Option<String>,
    },
    /// Prints data about a Pokémon move
    Move {
        /// Name of the move
        move_: String,
    },
    /// Prints data about a Pokémon ability
    Ability {
        /// Name of the ability
        ability: String,
    },
    /// Prints all possible names from a Resource such as Pokémon, Moves, etc
    Resource {
        /// Name of the resource
        #[arg(value_enum)]
        resource: ResourceArgs,
        /// Value to be printed in between values. Defaults to newline
        #[arg(short, long)]
        delimiter: Option<String>,
    },
    /// Dunspars configuration
    Config {
        /// Name of the target configuration. Prints all current config if empty
        key: Option<String>,
        /// Sets the target configuration to this value. Print its current value if empty
        value: Option<String>,
        /// Deletes the target configuration
        #[arg(short, long, action = clap::ArgAction::SetTrue)]
        unset: bool,
    },
}

#[derive(Clone, clap::ValueEnum)]
enum ResourceArgs {
    Pokemon,
    Moves,
    Abilities,
    Games,
    Types,
}

pub async fn run() -> Result<i32> {
    let cli = Cli::parse();

    let mut config_builder = ConfigBuilder::from_file()?;
    if let Some(game) = &cli.game {
        config_builder = config_builder.game(game.to_owned());
    }
    if cli.color {
        config_builder = config_builder.color_enabled(true);
    } else if cli.no_color {
        config_builder = config_builder.color_enabled(false);
    }
    let config = config_builder.build()?;

    let status_code = run_command(cli.command, config).await?;
    Ok(status_code)
}

async fn run_command(commands: Commands, config: Config) -> Result<i32> {
    let mut output = stdout().lock();

    // Performing dynamic dispatch with async functions may produce
    // obscure error messages which make it hard to debug.
    // Related issues:
    // https://github.com/rust-lang/rust/issues/119502
    // https://github.com/rust-lang/rust/issues/78649
    // https://github.com/rust-lang/rust/issues/119727
    match commands {
        Commands::Setup => {
            let cmd = SetupCommand;
            cmd.run(config, &mut output).await
        }
        Commands::Pokemon {
            pokemon,
            moves,
            evolution,
        } => {
            let cmd = PokemonCommand {
                name: pokemon,
                moves,
                evolution,
            };
            cmd.run(config, &mut output).await
        }
        Commands::Type {
            primary_type,
            secondary_type,
        } => {
            let cmd = TypeCommand {
                primary_type,
                secondary_type,
            };
            cmd.run(config, &mut output).await
        }
        Commands::Move { move_ } => {
            let cmd = MoveCommand { name: move_ };
            cmd.run(config, &mut output).await
        }
        Commands::Ability { ability } => {
            let cmd = AbilityCommand { name: ability };
            cmd.run(config, &mut output).await
        }
        Commands::Match {
            defenders,
            attacker,
            stab_only,
            verbose,
        } => {
            let cmd = MatchCommand {
                defender_names: defenders,
                attacker_name: attacker,
                stab_only,
                verbose,
            };
            cmd.run(config, &mut output).await
        }
        Commands::Coverage { pokemon } => {
            let cmd = CoverageCommand { names: pokemon };
            cmd.run(config, &mut output).await
        }
        Commands::Resource {
            resource,
            delimiter,
        } => {
            let cmd = ResourceCommand {
                resource,
                delimiter,
            };
            cmd.run(config, &mut output).await
        }
        Commands::Config { key, value, unset } => {
            let cmd = ConfigCommand { key, value, unset };
            cmd.run(config, &mut output).await
        }
    }
}
