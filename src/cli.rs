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
    Pokemon {
        name: String,
        #[arg(short, long)]
        version: Option<String>,
    },
    Type {
        name: String,
    },
}

pub async fn run() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Pokemon { name, version }) => run_pokemon(name, version).await?,
        Some(Commands::Type { name }) => run_type(name).await?,
        None => {}
    }

    Ok(())
}

async fn run_pokemon(name: String, version: Option<String>) -> Result<()> {
    let api = ApiWrapper::default();
    let version = version.unwrap_or("scarlet-violet".to_string());
    let pokemon = Pokemon::from_name(&api, &name, &version).await?;

    let defense_chart = pokemon.get_defense_chart().await?;
    let moves = pokemon.get_moves().await?;

    println!(
        "{}\nname: {}, types: {} {}",
        "pokemon".bright_green().bold(),
        pokemon.name,
        pokemon.primary_type,
        pokemon.secondary_type.as_ref().unwrap_or(&String::from(""))
    );

    println!(
        "\n{}\n{}",
        "defense chart".bright_green().bold(),
        defense_chart.group_by_multiplier()
    );

    println!(
        "\n{}\n{}",
        "moves".bright_green().bold(),
        moves
            .iter()
            .map(|mv| mv.to_string())
            .collect::<Vec<_>>()
            .join("\n")
    );
    Ok(())
}

async fn run_type(name: String) -> Result<()> {
    let api = ApiWrapper::default();
    let Type {
        offense_chart,
        defense_chart,
        ..
    } = Type::from_name(&api, &name).await?;

    println!(
        "{}\n{}\n",
        "offense chart".bright_green().bold(),
        offense_chart.group_by_multiplier()
    );
    println!(
        "{}\n{}\n",
        "defense chart".bright_green().bold(),
        defense_chart.group_by_multiplier()
    );

    Ok(())
}
