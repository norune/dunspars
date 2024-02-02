mod display;
mod utils;

use anyhow::Result;
use clap::{Parser, Subcommand};
use indoc::printdoc;

use crate::api::ApiWrapper;
use crate::pokemon::{Ability, Move, Pokemon, PokemonData, Type};
use display::*;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
    /// Specifies game version to pull data that is specific to a game or generation
    #[clap(short, long, global = true)]
    game: Option<String>,
}

#[derive(Subcommand)]
enum Commands {
    /// Displays general data about a Pokémon
    Pokemon {
        /// Name of the Pokémon
        name: String,
        /// Display all move data the Pokémon is capable of learning
        #[arg(short, long, action = clap::ArgAction::SetTrue)]
        moves: bool,
        /// Display the Pokémon evolutionary line
        #[arg(short, long, action = clap::ArgAction::SetTrue)]
        evolution: bool,
    },
    /// Displays matchup data between Pokémon
    Match {
        /// Names of the defending Pokémon; max 6
        #[arg(required = true, num_args = 1..=6)]
        defenders: Vec<String>,
        /// Name of the attacking Pokémon
        attacker: String,
        /// Display only moves that match the user's type
        #[arg(short, long, action = clap::ArgAction::SetTrue)]
        stab_only: bool,
    },
    /// Displays data about a Pokémon type
    Type {
        /// Name of the type
        name: String,
    },
    /// Displays data about a Pokémon move
    Move {
        /// Name of the move
        name: String,
    },
    /// Displays data about a Pokémon ability
    Ability {
        /// Name of the ability
        name: String,
    },
}

pub async fn run() -> Result<()> {
    let cli = Cli::parse();
    let version = cli.game.unwrap_or("scarlet-violet".to_string());
    let program = Program::new(version);

    match cli.command {
        Some(Commands::Pokemon {
            name,
            moves,
            evolution,
        }) => program.run_pokemon(name, moves, evolution).await?,
        Some(Commands::Type { name }) => program.run_type(name).await?,
        Some(Commands::Move { name }) => program.run_move(name).await?,
        Some(Commands::Ability { name }) => program.run_ability(name).await?,
        Some(Commands::Match {
            defenders,
            attacker,
            stab_only,
        }) => program.run_match(defenders, attacker, stab_only).await?,
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

    async fn run_pokemon(&self, name: String, moves: bool, evolution: bool) -> Result<()> {
        let api = ApiWrapper::default();

        let pokemon = PokemonData::from_name(&api, &name, &self.game).await?;
        let pokemon_display = PokemonDisplay::new(&pokemon);

        let defense_chart = pokemon.get_defense_chart().await?;
        let type_chart_display = TypeChartDisplay::new(&defense_chart, "defense chart");

        printdoc! {
            "
            {pokemon_display}

            {type_chart_display}
            "
        };

        if evolution {
            let evolution_step = pokemon.get_evolution_steps().await?;
            let evolution_step_display = EvolutionStepDisplay::new(&evolution_step);
            printdoc! {
                "

                {evolution_step_display}
                "
            };
        }

        if moves {
            let moves = pokemon.get_moves().await?;
            let move_list_display = MoveListDisplay::new(&moves, &pokemon);
            printdoc! {
                "

                {move_list_display}
                "
            };
        }

        Ok(())
    }

    async fn run_type(&self, name: String) -> Result<()> {
        let api = ApiWrapper::default();
        let generation = api.get_generation(&self.game).await?;
        let Type {
            offense_chart,
            defense_chart,
            ..
        } = Type::from_name(&api, &name, generation).await?;

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

    async fn run_match(
        &self,
        defenders: Vec<String>,
        attacker: String,
        stab_only: bool,
    ) -> Result<()> {
        let api = ApiWrapper::default();

        for defender in defenders {
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
        }

        Ok(())
    }

    async fn run_move(&self, name: String) -> Result<()> {
        let api = ApiWrapper::default();
        let generation = api.get_generation(&self.game).await?;
        let move_ = Move::from_name(&api, &name, generation).await?;
        let move_display = MoveDisplay::new(&move_);

        printdoc! {
            "
            {move_display}
            "
        };

        Ok(())
    }

    async fn run_ability(&self, name: String) -> Result<()> {
        let api = ApiWrapper::default();
        let generation = api.get_generation(&self.game).await?;
        let ability = Ability::from_name(&api, &name, generation).await?;
        let ability_display = AbilityDisplay::new(&ability);

        printdoc! {
            "
            {ability_display}
            "
        };

        Ok(())
    }
}
