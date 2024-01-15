use std::error::Error;

use clap::{Parser, Subcommand};
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

async fn view(pokemon: &str) -> Result<(), Box<dyn Error>> {
    let api_client = ApiWrapper::default();
    let pokemon = api_client.get_pokemon(pokemon).await?;
    let type_charts = api_client.get_type_charts(&pokemon.types.0, pokemon.types.1.as_deref()).await?;
    println!("name: {0}, types: {1} {2}", pokemon.name, pokemon.types.0, pokemon.types.1.unwrap_or("".to_string()));
    println!("defense chart: {:#?}", type_charts.1);

    Ok(())
}