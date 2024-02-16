mod display;
mod utils;

use anyhow::Result;
use clap::{Parser, Subcommand};
use indoc::printdoc;

use crate::api::resource::{
    AbilityResource, GameResource, MoveResource, PokemonResource, Resource, TypeResource,
};
use crate::api::ApiWrapper;
use crate::pokemon::{Ability, Move, Pokemon, PokemonData, Type};
use display::*;

const VERSION: &str = env!("DUNSPARS_VERSION");

#[derive(Parser)]
#[command(author, version = VERSION, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
    /// Sets the mainline Pokémon game the output will be based on
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
        resource: ResourceArgs,
        /// Value to be printed in between values. Defaults to newline
        #[arg(short, long)]
        delimiter: Option<String>,
    },
    /// Actions regarding the program's cache
    Cache {
        /// Action to undertake
        #[arg(value_enum)]
        action: CacheAction,
    },
}

#[derive(Clone, clap::ValueEnum)]
enum ResourceArgs {
    Pokemon,
    Moves,
    Abilities,
    Games,
    Types,
}

#[derive(Clone, clap::ValueEnum)]
enum CacheAction {
    Clear,
}

pub async fn run() -> Result<()> {
    let cli = Cli::parse();
    let api = ApiWrapper::try_new().await?;

    // Default to the latest game
    let game_name = cli
        .game
        .unwrap_or(api.game_resource.resource().last().unwrap().to_string());
    let game = api.game_resource.validate(&game_name)?;
    let generation = api.game_resource.get_gen(&game);

    let program = Program::new(game, generation, api);

    match cli.command {
        Commands::Pokemon {
            pokemon,
            moves,
            evolution,
        } => program.run_pokemon(pokemon, moves, evolution).await?,
        Commands::Type { type_ } => program.run_type(type_).await?,
        Commands::Move { move_ } => program.run_move(move_).await?,
        Commands::Ability { ability } => program.run_ability(ability).await?,
        Commands::Match {
            defenders,
            attacker,
            stab_only,
        } => program.run_match(defenders, attacker, stab_only).await?,
        Commands::Resource {
            resource,
            delimiter,
        } => program.run_resource(resource, delimiter).await?,
        Commands::Cache { action } => program.run_cache(action).await?,
    }

    Ok(())
}

struct Program {
    game: String,
    generation: u8,
    api: ApiWrapper,
}

impl Program {
    pub fn new(game: String, generation: u8, api: ApiWrapper) -> Self {
        Self {
            game,
            generation,
            api,
        }
    }

    async fn run_pokemon(&self, name: String, moves: bool, evolution: bool) -> Result<()> {
        let resource = PokemonResource::try_new(&self.api.client).await?;
        let pokemon_name = resource.validate(&name)?;

        let pokemon = PokemonData::from_name(&self.api, &pokemon_name, &self.game).await?;
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
        let resource = TypeResource::try_new(&self.api.client).await?;
        let type_name = resource.validate(&name)?;

        let Type {
            offense_chart,
            defense_chart,
            ..
        } = Type::from_name(&self.api, &type_name, self.generation).await?;

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
        let resource = PokemonResource::try_new(&self.api.client).await?;

        let attacker_name = resource.validate(&attacker_name)?;
        let attacker_data = PokemonData::from_name(&self.api, &attacker_name, &self.game).await?;
        let attacker_moves = attacker_data.get_moves().await?;
        let attacker_chart = attacker_data.get_defense_chart().await?;
        let attacker = Pokemon::new(attacker_data, attacker_chart, attacker_moves);

        let mut defenders = vec![];

        for defender_name in defender_names {
            let defender_name = resource.validate(&defender_name)?;
            let defender_data =
                PokemonData::from_name(&self.api, &defender_name, &self.game).await?;
            let defender_moves = defender_data.get_moves().await?;
            let defender_chart = defender_data.get_defense_chart().await?;
            let defender = Pokemon::new(defender_data, defender_chart, defender_moves);

            defenders.push(defender);
        }

        for defender in defenders {
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
        let resource = MoveResource::try_new(&self.api.client).await?;
        let move_name = resource.validate(&name)?;

        let move_ = Move::from_name(&self.api, &move_name, self.generation).await?;
        let move_display = MoveDisplay::new(&move_);

        printdoc! {
            "
            {move_display}
            "
        };

        Ok(())
    }

    async fn run_ability(&self, name: String) -> Result<()> {
        let resource = AbilityResource::try_new(&self.api.client).await?;
        let ability_name = resource.validate(&name)?;

        let ability = Ability::from_name(&self.api, &ability_name, self.generation).await?;
        let ability_display = AbilityDisplay::new(&ability);

        printdoc! {
            "
            {ability_display}
            "
        };

        Ok(())
    }

    async fn run_resource(&self, resource: ResourceArgs, delimiter: Option<String>) -> Result<()> {
        let delimiter = delimiter.unwrap_or("\n".to_string());
        let resource = match resource {
            ResourceArgs::Pokemon => PokemonResource::try_new(&self.api.client)
                .await?
                .resource()
                .join(&delimiter),
            ResourceArgs::Moves => MoveResource::try_new(&self.api.client)
                .await?
                .resource()
                .join(&delimiter),
            ResourceArgs::Abilities => AbilityResource::try_new(&self.api.client)
                .await?
                .resource()
                .join(&delimiter),
            ResourceArgs::Types => TypeResource::try_new(&self.api.client)
                .await?
                .resource()
                .join(&delimiter),
            ResourceArgs::Games => GameResource::try_new(&self.api.client)
                .await?
                .resource()
                .join(&delimiter),
        };

        printdoc! {
            "
            {resource}
            "
        };

        Ok(())
    }

    async fn run_cache(&self, action: CacheAction) -> Result<()> {
        match action {
            CacheAction::Clear => self.api.clear_cache().await,
        }
    }
}
