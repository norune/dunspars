use std::error::Error;

use clap::{Parser, Subcommand};
use crate::pokemon::Pokemon;
use crate::api::ApiWrapper;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    View {
        pokemon: String
    },
}

pub async fn run() -> Result<(), Box<dyn Error>> {
    let cli = Cli::parse();

    match &cli.command {
        Some(Commands::View { pokemon }) => view(&pokemon).await?,
        None => {}
    }

    Ok(())
}

async fn view(name: &str) -> Result<(), Box<dyn Error>> {
    let api = ApiWrapper::default();
    let pokemon = Pokemon::from_name(&api, name).await?;
    let defense_chart = pokemon.get_defense_chart().await?;

    println!("name: {0}, types: {1} {2}", pokemon.name, pokemon.primary_type, pokemon.secondary_type.unwrap_or("".to_string()));
    println!("defense chart: {:#?}", defense_chart);

    Ok(())
}