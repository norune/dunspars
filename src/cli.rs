mod display;

use anyhow::Result;
use clap::{Parser, Subcommand};
use owo_colors::OwoColorize;

use crate::api::ApiWrapper;
use crate::pokemon::Pokemon;
use crate::pokemon::Type;
use display::{MoveListDisplay, TypeChartDisplay};

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
    Match {
        name1: String,
        name2: String,
    },
    Type {
        name: String,
    },
}

pub async fn run() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Pokemon { name, version }) => run_pokemon(name, version).await?,
        Some(Commands::Match { name1, name2 }) => run_match(name1, name2).await?,
        Some(Commands::Type { name }) => run_type(name).await?,
        None => {}
    }

    Ok(())
}

async fn run_pokemon(name: String, version: Option<String>) -> Result<()> {
    let api = ApiWrapper::default();
    let version = version.unwrap_or("scarlet-violet".to_string());
    let pokemon = Pokemon::from_name(&api, &name, &version).await?;
    println!(
        "{}\nname: {}, types: {} {}",
        "pokemon".bright_green().bold(),
        pokemon.name,
        pokemon.primary_type,
        pokemon.secondary_type.as_ref().unwrap_or(&String::from(""))
    );

    let defense_chart = pokemon.get_defense_chart().await?;
    let type_chart_display = TypeChartDisplay::new(&defense_chart);
    println!("\n{}", "defense chart".bright_green().bold());
    type_chart_display.print_by_weakness()?;

    let moves = pokemon.get_moves().await?;
    let move_list_display = MoveListDisplay::new(&moves, &pokemon);
    println!("\n{}", "moves".bright_green().bold());
    move_list_display.print_list()?;

    Ok(())
}

async fn run_type(name: String) -> Result<()> {
    let api = ApiWrapper::default();
    let Type {
        offense_chart,
        defense_chart,
        ..
    } = Type::from_name(&api, &name).await?;

    let offense_chart_display = TypeChartDisplay::new(&offense_chart);
    let defense_chart_display = TypeChartDisplay::new(&defense_chart);

    println!("\n{}\n", "offense chart".bright_green().bold());
    offense_chart_display.print_by_weakness()?;
    println!("\n{}\n", "defense chart".bright_green().bold());
    defense_chart_display.print_by_weakness()?;

    Ok(())
}

async fn run_match(name1: String, name2: String) -> Result<()> {
    let api = ApiWrapper::default();
    let version = "scarlet-violet".to_string();
    let pokemon1 = Pokemon::from_name(&api, &name1, &version).await?;
    let pokemon2 = Pokemon::from_name(&api, &name2, &version).await?;

    pokemon1.match_up(&pokemon2).await?;

    Ok(())
}
