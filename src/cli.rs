mod display;
mod utils;

use anyhow::{anyhow, Result};
use clap::{Parser, Subcommand};
use indoc::{formatdoc, printdoc};

use crate::api::resource::{
    AbilityResource, GameResource, GetGeneration, MoveResource, PokemonResource, Resource,
    TypeResource,
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
    /// Force output to include colors
    #[clap(long, action = clap::ArgAction::SetTrue, global = true)]
    color: bool,
    /// Force output to exclude colors
    #[clap(long, action = clap::ArgAction::SetTrue, global = true)]
    no_color: bool,
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

    let mut config_builder = ConfigBuilder::new(&api.game_resource);
    if let Some(game) = cli.game {
        config_builder = config_builder.game(game);
    }
    if cli.color {
        config_builder = config_builder.color_enabled(true);
    }
    if cli.no_color {
        config_builder = config_builder.color_enabled(false);
    }
    let config = config_builder.build()?;

    let program = Program::new(config, api);

    match cli.command {
        Commands::Pokemon {
            pokemon,
            moves,
            evolution,
        } => print!("{}", program.run_pokemon(pokemon, moves, evolution).await?),
        Commands::Type { type_ } => print!("{}", program.run_type(type_).await?),
        Commands::Move { move_ } => print!("{}", program.run_move(move_).await?),
        Commands::Ability { ability } => print!("{}", program.run_ability(ability).await?),
        Commands::Match {
            defenders,
            attacker,
            stab_only,
        } => print!(
            "{}",
            program.run_match(defenders, attacker, stab_only).await?
        ),
        Commands::Resource {
            resource,
            delimiter,
        } => program.run_resource(resource, delimiter).await?,
        Commands::Cache { action } => program.run_cache(action).await?,
    }

    Ok(())
}

struct ConfigBuilder<'a> {
    game: Option<String>,
    color_enabled: Option<bool>,
    game_resource: &'a GameResource,
}

impl<'a> ConfigBuilder<'a> {
    pub fn new(game_resource: &'a GameResource) -> Self {
        ConfigBuilder {
            game_resource,
            game: None,
            color_enabled: None,
        }
    }

    pub fn game(mut self, game: String) -> Self {
        self.game = Some(game);
        self
    }

    pub fn color_enabled(mut self, color_enabled: bool) -> Self {
        self.color_enabled = Some(color_enabled);
        self
    }

    pub fn build(self) -> Result<Config> {
        let game = match self.game {
            Some(game) => self.game_resource.validate(&game)?,
            None => self
                .get_latest_game()
                .ok_or(anyhow!("Cannot find the latest game"))?,
        };

        let generation = self.game_resource.get_gen(&game);
        let color_enabled = self.color_enabled.unwrap_or(utils::is_color_enabled());

        Ok(Config {
            game,
            generation,
            color_enabled,
        })
    }

    fn get_latest_game(&self) -> Option<String> {
        self.game_resource.resource().last().map(|g| g.to_string())
    }
}

struct Config {
    game: String,
    generation: u8,
    color_enabled: bool,
}

struct Program {
    config: Config,
    api: ApiWrapper,
}

impl Program {
    pub fn new(config: Config, api: ApiWrapper) -> Self {
        Self { config, api }
    }

    async fn run_pokemon(&self, name: String, moves: bool, evolution: bool) -> Result<String> {
        let resource = PokemonResource::try_new(&self.api.client).await?;
        let pokemon_name = resource.validate(&name)?;

        let pokemon = PokemonData::from_name(&self.api, &pokemon_name, &self.config.game).await?;
        let pokemon_display = PokemonDisplay::new(&pokemon, self.config.color_enabled);

        let defense_chart = pokemon.get_defense_chart().await?;
        let type_chart_display = TypeChartDisplay::new(
            &defense_chart,
            "defenses".to_string(),
            self.config.color_enabled,
        );

        let mut output = formatdoc! {
            "
            {pokemon_display}

            {type_chart_display}
            "
        };

        if evolution {
            let evolution_step = pokemon.get_evolution_steps().await?;
            let evolution_step_display =
                EvolutionStepDisplay::new(&evolution_step, self.config.color_enabled);
            output += formatdoc! {
                "

                {evolution_step_display}
                "
            }
            .as_str();
        }

        if moves {
            let moves = pokemon.get_moves().await?;
            let move_list_display =
                MoveListDisplay::new(&moves, &pokemon, self.config.color_enabled);
            output += formatdoc! {
                "

                {move_list_display}
                "
            }
            .as_str();
        }

        Ok(output)
    }

    async fn run_type(&self, name: String) -> Result<String> {
        let resource = TypeResource::try_new(&self.api.client).await?;
        let type_name = resource.validate(&name)?;

        let Type {
            offense_chart,
            defense_chart,
            ..
        } = Type::from_name(&self.api, &type_name, self.config.generation).await?;

        let offense_chart_display = TypeChartDisplay::new(
            &offense_chart,
            format!("{type_name} offense"),
            self.config.color_enabled,
        );
        let defense_chart_display = TypeChartDisplay::new(
            &defense_chart,
            format!("{type_name} defense"),
            self.config.color_enabled,
        );

        let output = formatdoc! {
            "
            {offense_chart_display}

            {defense_chart_display}
            "
        };

        Ok(output)
    }

    async fn run_match(
        &self,
        defender_names: Vec<String>,
        attacker_name: String,
        stab_only: bool,
    ) -> Result<String> {
        let resource = PokemonResource::try_new(&self.api.client).await?;

        let attacker_name = resource.validate(&attacker_name)?;
        let attacker_data =
            PokemonData::from_name(&self.api, &attacker_name, &self.config.game).await?;
        let attacker_moves = attacker_data.get_moves().await?;
        let attacker_chart = attacker_data.get_defense_chart().await?;
        let attacker = Pokemon::new(attacker_data, attacker_chart, attacker_moves);

        let mut defenders = vec![];

        for defender_name in defender_names {
            let defender_name = resource.validate(&defender_name)?;
            let defender_data =
                PokemonData::from_name(&self.api, &defender_name, &self.config.game).await?;
            let defender_moves = defender_data.get_moves().await?;
            let defender_chart = defender_data.get_defense_chart().await?;
            let defender = Pokemon::new(defender_data, defender_chart, defender_moves);

            defenders.push(defender);
        }

        let mut output = String::from("");
        for defender in defenders {
            let match_display =
                MatchDisplay::new(&defender, &attacker, stab_only, self.config.color_enabled);
            output += formatdoc! {
                "
                {match_display}


                "
            }
            .as_str();
        }

        Ok(output)
    }

    async fn run_move(&self, name: String) -> Result<String> {
        let resource = MoveResource::try_new(&self.api.client).await?;
        let move_name = resource.validate(&name)?;

        let move_ = Move::from_name(&self.api, &move_name, self.config.generation).await?;
        let move_display = MoveDisplay::new(&move_, self.config.color_enabled);

        let output = formatdoc! {
            "
            {move_display}
            "
        };

        Ok(output)
    }

    async fn run_ability(&self, name: String) -> Result<String> {
        let resource = AbilityResource::try_new(&self.api.client).await?;
        let ability_name = resource.validate(&name)?;

        let ability = Ability::from_name(&self.api, &ability_name, self.config.generation).await?;
        let ability_display = AbilityDisplay::new(&ability, self.config.color_enabled);

        let output = formatdoc! {
            "
            {ability_display}
            "
        };

        Ok(output)
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

#[cfg(test)]
mod tests {
    use super::*;

    async fn setup_program(game: &str) -> Program {
        let api = ApiWrapper::try_new().await.unwrap();
        let config = ConfigBuilder::new(&api.game_resource)
            .game(String::from(game))
            .color_enabled(false)
            .build()
            .unwrap();
        Program::new(config, api)
    }

    #[tokio::test]
    async fn run_pokemon() {
        let program = setup_program("scarlet-violet").await;

        let output = program
            .run_pokemon(String::from("ceruledge"), false, false)
            .await
            .unwrap();

        insta::with_settings!({
            description => "pokemon ceruledge --game scarlet-violet",
            omit_expression => true
        }, {
            insta::assert_snapshot!(output);
        });
    }

    #[tokio::test]
    async fn run_pokemon_evolution() {
        let program = setup_program("sword-shield").await;

        let output = program
            .run_pokemon(String::from("cascoon"), false, true)
            .await
            .unwrap();

        insta::with_settings!({
            description => "pokemon cascoon -e --game sword-shield",
            omit_expression => true
        }, {
            insta::assert_snapshot!(output);
        });
    }

    #[tokio::test]
    async fn run_pokemon_moves() {
        let program = setup_program("scarlet-violet").await;

        let output = program
            .run_pokemon(String::from("blaziken"), true, false)
            .await
            .unwrap();

        insta::with_settings!({
            description => "pokemon blaziken -m --game scarlet-violet",
            omit_expression => true
        }, {
            insta::assert_snapshot!(output);
        });
    }

    #[tokio::test]
    async fn run_type() {
        let program = setup_program("platinum").await;

        let output = program.run_type(String::from("ice")).await.unwrap();

        insta::with_settings!({
            description => "type ice --game platinum",
            omit_expression => true
        }, {
            insta::assert_snapshot!(output);
        });
    }

    #[tokio::test]
    async fn run_move() {
        let program = setup_program("sun-moon").await;

        let output = program.run_move(String::from("brick-break")).await.unwrap();

        insta::with_settings!({
            description => "move brick-break --game sun-moon",
            omit_expression => true
        }, {
            insta::assert_snapshot!(output);
        });
    }

    #[tokio::test]
    async fn run_ability() {
        let program = setup_program("black-white").await;

        let output = program
            .run_ability(String::from("intimidate"))
            .await
            .unwrap();

        insta::with_settings!({
            description => "ability intimidate --game black-white",
            omit_expression => true
        }, {
            insta::assert_snapshot!(output);
        });
    }

    #[tokio::test]
    async fn run_match() {
        let program = setup_program("x-y").await;

        let output = program
            .run_match(
                vec![String::from("golem"), String::from("pachirisu")],
                String::from("lapras"),
                false,
            )
            .await
            .unwrap();

        insta::with_settings!({
            description => "match golem pachirisu lapras --game x-y",
            omit_expression => true
        }, {
            insta::assert_snapshot!(output);
        });
    }
}
