use anyhow::{Result, Ok};
use clap::{Parser, Subcommand};

use crate::pokemon::Pokemon;
use crate::pokemon::Type;
use crate::api::ApiWrapper;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    Pokemon {
        name: String
    },
    Type {
        name: String
    }
}

pub async fn run() -> Result<()> {
    let cli = Cli::parse();

    match &cli.command {
        Some(Commands::Pokemon { name }) => run_pokemon(&name).await?,
        Some(Commands::Type { name }) => run_type(&name).await?,
        None => {}
    }

    Ok(())
}

async fn run_pokemon(name: &str) -> Result<()> {
    let api = ApiWrapper::default();
    let pokemon = Pokemon::from_name(&api, name).await?;
    let defense_chart = pokemon.get_defense_chart().await?;

    println!("pokemon\nname: {0}, types: {1} {2}", pokemon.name, pokemon.primary_type, pokemon.secondary_type.unwrap_or("".to_string()));
    println!("\ndefense chart\n{}", defense_chart);

    Ok(())
}

async fn run_type(name: &str) -> Result<()> {
    let api = ApiWrapper::default();
    let type_ = Type::from_name(&api, name).await?;

    println!("offense chart\n{}\n", type_.offense_chart);
    println!("defense chart\n{}\n", type_.defense_chart);

    Ok(())
}