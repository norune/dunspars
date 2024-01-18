use anyhow::{Ok, Result};
use clap::{Parser, Subcommand};
use owo_colors::OwoColorize;

use crate::api::ApiWrapper;
use crate::pokemon::Pokemon;
use crate::pokemon::Type;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    Pokemon { name: String },
    Type { name: String },
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

    println!(
        "{}\nname: {}, types: {} {}",
        "pokemon".bright_green(),
        pokemon.name,
        pokemon.primary_type,
        pokemon.secondary_type.unwrap_or("".to_string())
    );

    println!(
        "\n{}\n{}",
        "defense chart".bright_green(),
        defense_chart.group_by_multiplier()
    );

    Ok(())
}

async fn run_type(name: &str) -> Result<()> {
    let api = ApiWrapper::default();
    let Type {
        offense_chart,
        defense_chart,
        ..
    } = Type::from_name(&api, name).await?;

    println!(
        "{}\n{}\n",
        "offense chart".bright_green(),
        offense_chart.group_by_multiplier()
    );
    println!(
        "{}\n{}\n",
        "defense chart".bright_green(),
        defense_chart.group_by_multiplier()
    );

    Ok(())
}
