mod display;

use anyhow::Result;
use clap::{Parser, Subcommand};
use owo_colors::OwoColorize;

use crate::api::ApiWrapper;
use crate::pokemon::{Move, Pokemon, Type};
use display::{MatchDisplay, MoveDisplay, MoveListDisplay, TypeChartDisplay};

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
        #[arg(short, long, action = clap::ArgAction::SetTrue)]
        reverse: bool,
        #[arg(short, long, action = clap::ArgAction::SetTrue)]
        stab_only: bool,
        defender: String,
        attacker: String,
    },
    Type {
        name: String,
    },
    Move {
        name: String,
    },
}

pub async fn run() -> Result<()> {
    let program = Program::new();
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Pokemon { name, version }) => program.run_pokemon(name, version).await?,
        Some(Commands::Type { name }) => program.run_type(name).await?,
        Some(Commands::Move { name }) => program.run_move(name).await?,
        Some(Commands::Match {
            attacker,
            defender,
            reverse,
            stab_only,
        }) => {
            let pokemon = if reverse {
                (attacker, defender)
            } else {
                (defender, attacker)
            };

            program.run_match(pokemon.0, pokemon.1, stab_only).await?
        }
        None => {}
    }

    Ok(())
}

struct Program {}

impl Program {
    pub fn new() -> Program {
        Program {}
    }

    async fn run_pokemon(&self, name: String, version: Option<String>) -> Result<()> {
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
        type_chart_display.print()?;

        let moves = pokemon.get_moves().await?;
        let move_list_display = MoveListDisplay::new(&moves, &pokemon);
        move_list_display.print()?;

        Ok(())
    }

    async fn run_type(&self, name: String) -> Result<()> {
        let api = ApiWrapper::default();
        let Type {
            offense_chart,
            defense_chart,
            ..
        } = Type::from_name(&api, &name).await?;

        let offense_chart_display = TypeChartDisplay::new(&offense_chart);
        let defense_chart_display = TypeChartDisplay::new(&defense_chart);

        println!("\n{}", "offense chart".bright_green().bold());
        offense_chart_display.print()?;
        println!("\n{}", "defense chart".bright_green().bold());
        defense_chart_display.print()?;

        Ok(())
    }

    async fn run_match(&self, defender: String, attacker: String, stab_only: bool) -> Result<()> {
        let api = ApiWrapper::default();
        let version = "scarlet-violet".to_string();
        let defender = Pokemon::from_name(&api, &defender, &version).await?;
        let attacker = Pokemon::from_name(&api, &attacker, &version).await?;

        let defense_chart = defender.get_defense_chart().await?;
        let move_list = attacker.get_moves().await?;

        let move_weak_display = MatchDisplay::new(&defense_chart, &move_list, &attacker, stab_only);
        println!("\n{}", "weaknesses by moves".bright_green().bold());
        move_weak_display.print()?;

        Ok(())
    }

    async fn run_move(&self, name: String) -> Result<()> {
        let api = ApiWrapper::default();
        let move_ = Move::from_name(&api, &name).await?;
        let display = MoveDisplay::new(&move_);
        display.print()?;

        Ok(())
    }
}
