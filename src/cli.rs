mod display;
mod utils;

use anyhow::Result;
use clap::{Parser, Subcommand};
use indoc::printdoc;

use crate::api::ApiWrapper;
use crate::pokemon::{Move, Pokemon, PokemonData, Type};
use display::*;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
    #[arg(short, long)]
    game: Option<String>,
}

#[derive(Subcommand)]
enum Commands {
    Pokemon {
        name: String,
    },
    Match {
        defender: String,
        attacker: String,
        #[arg(short, long, action = clap::ArgAction::SetTrue)]
        stab_only: bool,
    },
    Type {
        name: String,
    },
    Move {
        name: String,
    },
}

pub async fn run() -> Result<()> {
    let cli = Cli::parse();
    let version = cli.game.unwrap_or("scarlet-violet".to_string());
    let program = Program::new(version);

    match cli.command {
        Some(Commands::Pokemon { name }) => program.run_pokemon(name).await?,
        Some(Commands::Type { name }) => program.run_type(name).await?,
        Some(Commands::Move { name }) => program.run_move(name).await?,
        Some(Commands::Match {
            defender,
            attacker,
            stab_only,
        }) => program.run_match(defender, attacker, stab_only).await?,
        None => {}
    }

    Ok(())
}

struct Program {
    game: String,
}

impl Program {
    pub fn new(version: String) -> Self {
        Self { game: version }
    }

    async fn run_pokemon(&self, name: String) -> Result<()> {
        let api = ApiWrapper::default();

        let pokemon = PokemonData::from_name(&api, &name, &self.game).await?;
        let pokemon_display = PokemonDisplay::new(&pokemon);

        let defense_chart = pokemon.get_defense_chart().await?;
        let type_chart_display = TypeChartDisplay::new(&defense_chart, "defense chart");

        let moves = pokemon.get_moves().await?;
        let move_list_display = MoveListDisplay::new(&moves, &pokemon);

        printdoc! {
            "
            {pokemon_display}

            {type_chart_display}

            {move_list_display}
            "
        };

        Ok(())
    }

    async fn run_type(&self, name: String) -> Result<()> {
        let api = ApiWrapper::default();
        let Type {
            offense_chart,
            defense_chart,
            ..
        } = Type::from_name(&api, &name).await?;

        let offense_chart_display = TypeChartDisplay::new(&offense_chart, "offense chart");
        let defense_chart_display = TypeChartDisplay::new(&defense_chart, "defense chart");

        printdoc! {
            "
            {offense_chart_display}

            {defense_chart_display}
            "
        };

        Ok(())
    }

    async fn run_match(&self, defender: String, attacker: String, stab_only: bool) -> Result<()> {
        let api = ApiWrapper::default();

        let defender_data = PokemonData::from_name(&api, &defender, &self.game).await?;
        let defender_moves = defender_data.get_moves().await?;
        let defender_chart = defender_data.get_defense_chart().await?;
        let defender = Pokemon::new(defender_data, defender_chart, defender_moves);

        let attacker_data = PokemonData::from_name(&api, &attacker, &self.game).await?;
        let attacker_moves = attacker_data.get_moves().await?;
        let attacker_chart = attacker_data.get_defense_chart().await?;
        let attacker = Pokemon::new(attacker_data, attacker_chart, attacker_moves);

        let match_display = MatchDisplay::new(&defender, &attacker, stab_only);

        printdoc! {
            "
            {match_display}
            "
        };

        Ok(())
    }

    async fn run_move(&self, name: String) -> Result<()> {
        let api = ApiWrapper::default();
        let move_ = Move::from_name(&api, &name).await?;
        let move_display = MoveDisplay::new(&move_);

        printdoc! {
            "
            {move_display}
            "
        };

        Ok(())
    }
}
