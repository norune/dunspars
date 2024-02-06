mod display;
mod utils;

use anyhow::Result;
use clap::{Parser, Subcommand};
use indoc::printdoc;

use crate::api::ApiWrapper;
use crate::pokemon::{
    Ability, AbilityName, GameName, Move, MoveName, Pokemon, PokemonData, PokemonName,
    ResourceName, Type, TypeName,
};
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
    },
    /// Prints data about a Pokémon type
    Type {
        /// Name of the type
        type_: String,
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
        resource: Resource,
        /// Value to be printed in between values. Defaults to newline
        #[arg(short, long)]
        delimiter: Option<String>,
    },
}

#[derive(Clone, clap::ValueEnum)]
pub enum Resource {
    Pokemon,
    Moves,
    Abilities,
    Games,
    Types,
}

impl Resource {
    async fn get_resource(&self, api: &ApiWrapper) -> Result<Vec<String>> {
        match self {
            Resource::Pokemon => Ok(api.get_all_pokemon().await?),
            Resource::Moves => Ok(api.get_all_moves().await?),
            Resource::Abilities => Ok(api.get_all_abilities().await?),
            Resource::Types => Ok(api.get_all_types().await?),
            Resource::Games => Ok(api.get_all_games().await?),
        }
    }
}

pub async fn run() -> Result<()> {
    let cli = Cli::parse();
    let api = ApiWrapper::try_new()?;

    let game_resource = Resource::Games.get_resource(&api).await?;
    let game_name = cli.game.unwrap_or("scarlet-violet".to_string());
    let game = GameName::try_new(&game_name, &game_resource)?;
    let generation = api.get_generation(game.get()).await?;

    let program = Program::new(game, generation, api);

    match cli.command {
        Some(Commands::Pokemon {
            pokemon,
            moves,
            evolution,
        }) => program.run_pokemon(pokemon, moves, evolution).await?,
        Some(Commands::Type { type_ }) => program.run_type(type_).await?,
        Some(Commands::Move { move_ }) => program.run_move(move_).await?,
        Some(Commands::Ability { ability }) => program.run_ability(ability).await?,
        Some(Commands::Match {
            defenders,
            attacker,
            stab_only,
        }) => program.run_match(defenders, attacker, stab_only).await?,
        Some(Commands::Resource {
            resource,
            delimiter,
        }) => program.run_resource(resource, delimiter).await?,
        None => {}
    }

    Ok(())
}

struct Program {
    game: GameName,
    generation: u8,
    api: ApiWrapper,
}

impl Program {
    pub fn new(game: GameName, generation: u8, api: ApiWrapper) -> Self {
        Self {
            game,
            generation,
            api,
        }
    }

    async fn run_pokemon(&self, name: String, moves: bool, evolution: bool) -> Result<()> {
        let resource = Resource::Pokemon.get_resource(&self.api).await?;
        let pokemon_name = PokemonName::try_new(&name, &resource)?;

        let pokemon =
            PokemonData::from_name(&self.api, pokemon_name.get(), self.game.get()).await?;
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
        let resource = Resource::Types.get_resource(&self.api).await?;
        let type_name = TypeName::try_new(&name, &resource)?;

        let Type {
            offense_chart,
            defense_chart,
            ..
        } = Type::from_name(&self.api, type_name.get(), self.generation).await?;

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
        defender_names: Vec<String>,
        attacker_name: String,
        stab_only: bool,
    ) -> Result<()> {
        let resource = Resource::Pokemon.get_resource(&self.api).await?;

        for defender_name in defender_names {
            let defender_name = PokemonName::try_new(&defender_name, &resource)?;
            let defender_data =
                PokemonData::from_name(&self.api, defender_name.get(), self.game.get()).await?;
            let defender_moves = defender_data.get_moves().await?;
            let defender_chart = defender_data.get_defense_chart().await?;
            let defender = Pokemon::new(defender_data, defender_chart, defender_moves);

            let attacker_name = PokemonName::try_new(&attacker_name, &resource)?;
            let attacker_data =
                PokemonData::from_name(&self.api, attacker_name.get(), self.game.get()).await?;
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
        let resource = Resource::Moves.get_resource(&self.api).await?;
        let move_name = MoveName::try_new(&name, &resource)?;

        let move_ = Move::from_name(&self.api, move_name.get(), self.generation).await?;
        let move_display = MoveDisplay::new(&move_);

        printdoc! {
            "
            {move_display}
            "
        };

        Ok(())
    }

    async fn run_ability(&self, name: String) -> Result<()> {
        let resource = Resource::Abilities.get_resource(&self.api).await?;
        let ability_name = AbilityName::try_new(&name, &resource)?;

        let ability = Ability::from_name(&self.api, ability_name.get(), self.generation).await?;
        let ability_display = AbilityDisplay::new(&ability);

        printdoc! {
            "
            {ability_display}
            "
        };

        Ok(())
    }

    async fn run_resource(&self, resource: Resource, delimiter: Option<String>) -> Result<()> {
        let delimiter = delimiter.unwrap_or("\n".to_string());
        let resource = resource.get_resource(&self.api).await?.join(&delimiter);

        printdoc! {
            "
            {resource}
            "
        };

        Ok(())
    }
}
