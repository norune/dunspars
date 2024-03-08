mod display;
mod utils;

use crate::api;
use crate::api::once::{api_client, game_resource};
use crate::models::{Ability, Move, Pokemon, PokemonData, Type};
use crate::resource::{
    AbilityResource, DatabaseFile, GameResourceFile, GetGeneration, MoveResource, PokemonResource,
    Resource, ResourceFile, TypeResource,
};
use display::*;

use anyhow::{anyhow, Result};
use clap::{Parser, Subcommand};
use indoc::{formatdoc, printdoc};

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
    /// Retrieve and set up program data. Run this before using the program
    Setup,
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
        /// Display verbose output
        #[arg(short, long, action = clap::ArgAction::SetTrue)]
        verbose: bool,
    },
    /// Prints type coverage based on the provided Pokémon
    Coverage {
        /// Names of Pokémon; max 6
        #[arg(required = true, num_args = 1..=6)]
        pokemon: Vec<String>,
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
    /// Reset program files
    Reset {
        /// Which program data to reset
        #[arg(value_enum)]
        type_: ResetType,
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
enum ResetType {
    Cache,
    Data,
}

pub async fn run() -> Result<()> {
    let cli = Cli::parse();

    let game_resource_file = GameResourceFile::try_new()?;
    game_resource_file.build_if_missing(false).await?;
    let db_file = DatabaseFile::try_new(false)?;

    let mut config_builder = ConfigBuilder::new();
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

    let program = Program::new(config, db_file);

    match cli.command {
        Commands::Setup => program.run_setup().await?,
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
            verbose,
        } => print!(
            "{}",
            program
                .run_match(defenders, attacker, verbose, stab_only)
                .await?
        ),
        Commands::Coverage { pokemon } => print!("{}", program.run_coverage(pokemon).await?),
        Commands::Resource {
            resource,
            delimiter,
        } => program.run_resource(resource, delimiter).await?,
        Commands::Reset { type_ } => program.run_reset(type_).await?,
    }

    Ok(())
}

struct ConfigBuilder {
    game: Option<String>,
    color_enabled: Option<bool>,
}

impl ConfigBuilder {
    pub fn new() -> Self {
        ConfigBuilder {
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
            Some(game) => game_resource().validate(&game)?,
            None => self
                .get_latest_game()
                .ok_or(anyhow!("Cannot find the latest game"))?,
        };

        let generation = game_resource().get_gen(&game);
        let color_enabled = self.color_enabled.unwrap_or(utils::is_color_enabled());

        Ok(Config {
            game,
            generation,
            color_enabled,
        })
    }

    fn get_latest_game(&self) -> Option<String> {
        game_resource().resource().last().map(|g| g.to_string())
    }
}

struct Config {
    game: String,
    generation: u8,
    color_enabled: bool,
}

struct Program {
    config: Config,
    db_file: DatabaseFile,
}

impl Program {
    pub fn new(config: Config, db_file: DatabaseFile) -> Self {
        Self { config, db_file }
    }

    async fn run_pokemon(&self, name: String, moves: bool, evolution: bool) -> Result<String> {
        let resource = PokemonResource::try_new(api_client()).await?;
        let pokemon_name = resource.validate(&name)?;

        let pokemon = PokemonData::from_name(&pokemon_name, &self.config.game).await?;
        let pokemon_display = DisplayComponent::new(&pokemon, self.config.color_enabled);

        let defense_chart = pokemon.get_defense_chart(&self.db_file.db)?;
        let defense_chart_ctx = TypeChartComponent {
            type_chart: &defense_chart,
        };
        let type_chart_display =
            DisplayComponent::new(defense_chart_ctx, self.config.color_enabled);

        let mut output = formatdoc! {
            "
            {pokemon_display}

            {type_chart_display}
            "
        };

        if evolution {
            let evolution_step = pokemon.get_evolution_steps().await?;
            let evolution_step_display =
                DisplayComponent::new(&evolution_step, self.config.color_enabled);
            output += formatdoc! {
                "

                {evolution_step_display}
                "
            }
            .as_str();
        }

        if moves {
            let moves = pokemon.get_moves(&self.db_file.db)?;
            let move_list_context = MoveListComponent {
                move_list: &moves,
                pokemon: &pokemon,
            };
            let move_list_display =
                DisplayComponent::new(move_list_context, self.config.color_enabled);

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
        let resource = TypeResource::try_new(api_client()).await?;
        let type_name = resource.validate(&name)?;

        let Type {
            offense_chart,
            defense_chart,
            ..
        } = Type::from_name(&type_name, self.config.generation, &self.db_file.db)?;

        let offense_chart_ctx = TypeChartComponent {
            type_chart: &offense_chart,
        };
        let offense_chart_display =
            DisplayComponent::new(offense_chart_ctx, self.config.color_enabled);

        let defense_chart_ctx = TypeChartComponent {
            type_chart: &defense_chart,
        };
        let defense_chart_display =
            DisplayComponent::new(defense_chart_ctx, self.config.color_enabled);

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
        verbose: bool,
        stab_only: bool,
    ) -> Result<String> {
        let resource = PokemonResource::try_new(api_client()).await?;

        let attacker_name = resource.validate(&attacker_name)?;
        let attacker_data = PokemonData::from_name(&attacker_name, &self.config.game).await?;
        let attacker_moves = attacker_data.get_moves(&self.db_file.db)?;
        let attacker_chart = attacker_data.get_defense_chart(&self.db_file.db)?;
        let attacker = Pokemon::new(attacker_data, attacker_chart, attacker_moves);

        let mut defenders = vec![];

        for defender_name in defender_names {
            let defender_name = resource.validate(&defender_name)?;
            let defender_data = PokemonData::from_name(&defender_name, &self.config.game).await?;
            let defender_moves = defender_data.get_moves(&self.db_file.db)?;
            let defender_chart = defender_data.get_defense_chart(&self.db_file.db)?;
            let defender = Pokemon::new(defender_data, defender_chart, defender_moves);

            defenders.push(defender);
        }

        let mut output = String::from("");
        for defender in defenders {
            let match_context = MatchComponent {
                defender: &defender,
                attacker: &attacker,
                verbose,
                stab_only,
            };
            let match_display = DisplayComponent::new(match_context, self.config.color_enabled);

            output += formatdoc! {
                "
                {match_display}


                "
            }
            .as_str();
        }

        Ok(output)
    }

    async fn run_coverage(&self, names: Vec<String>) -> Result<String> {
        let resource = PokemonResource::try_new(api_client()).await?;
        let mut pokemon = vec![];

        for name in names {
            let name = resource.validate(&name)?;
            let data = PokemonData::from_name(&name, &self.config.game).await?;
            let moves = data.get_moves(&self.db_file.db)?;
            let chart = data.get_defense_chart(&self.db_file.db)?;

            let mon = Pokemon::new(data, chart, moves);
            pokemon.push(mon);
        }

        let coverage_ctx = CoverageComponent {
            pokemon: &pokemon,
            db: &self.db_file.db,
        };
        let coverage_display = DisplayComponent::new(coverage_ctx, self.config.color_enabled);

        Ok(formatdoc! {
            "
            {coverage_display}
            "
        })
    }

    async fn run_move(&self, name: String) -> Result<String> {
        let resource = MoveResource::try_new(api_client()).await?;
        let move_name = resource.validate(&name)?;

        let move_ = Move::from_name(&move_name, self.config.generation, &self.db_file.db)?;
        let move_display = DisplayComponent::new(&move_, self.config.color_enabled);

        let output = formatdoc! {
            "
            {move_display}
            "
        };

        Ok(output)
    }

    async fn run_ability(&self, name: String) -> Result<String> {
        let resource = AbilityResource::try_new(api_client()).await?;
        let ability_name = resource.validate(&name)?;

        let ability = Ability::from_name(&ability_name, self.config.generation).await?;
        let ability_display = DisplayComponent::new(&ability, self.config.color_enabled);

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
            ResourceArgs::Pokemon => PokemonResource::try_new(api_client())
                .await?
                .resource()
                .join(&delimiter),
            ResourceArgs::Moves => MoveResource::try_new(api_client())
                .await?
                .resource()
                .join(&delimiter),
            ResourceArgs::Abilities => AbilityResource::try_new(api_client())
                .await?
                .resource()
                .join(&delimiter),
            ResourceArgs::Types => TypeResource::try_new(api_client())
                .await?
                .resource()
                .join(&delimiter),
            ResourceArgs::Games => game_resource().resource().join(&delimiter),
        };

        printdoc! {
            "
            {resource}
            "
        };

        Ok(())
    }

    async fn run_reset(&self, action: ResetType) -> Result<()> {
        match action {
            ResetType::Cache => api::clear_cache().await,
            ResetType::Data => {
                let game_writer = GameResourceFile::try_new()?;
                game_writer.build_if_missing(true).await?;
                Ok(())
            }
        }
    }

    async fn run_setup(&self) -> Result<()> {
        let db = DatabaseFile::try_new(true)?;
        db.build_db().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn setup_program(game: &str) -> Program {
        let config = ConfigBuilder::new()
            .game(String::from(game))
            .color_enabled(false)
            .build()
            .unwrap();
        let db_file = DatabaseFile::try_new(false).unwrap();
        Program::new(config, db_file)
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

        let cascoon = program
            .run_pokemon(String::from("cascoon"), false, true)
            .await
            .unwrap();

        insta::with_settings!({
            description => "pokemon cascoon --evolution --game sword-shield",
            omit_expression => true
        }, {
            insta::assert_snapshot!(cascoon);
        });

        let politoed = program
            .run_pokemon(String::from("politoed"), false, true)
            .await
            .unwrap();

        insta::with_settings!({
            description => "pokemon politoed --evolution --game sword-shield",
            omit_expression => true
        }, {
            insta::assert_snapshot!(politoed);
        });

        let applin = program
            .run_pokemon(String::from("applin"), false, true)
            .await
            .unwrap();

        insta::with_settings!({
            description => "pokemon applin --evolution --game sword-shield",
            omit_expression => true
        }, {
            insta::assert_snapshot!(applin);
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
            description => "pokemon blaziken --moves --game scarlet-violet",
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
        let defenders = vec![String::from("golem"), String::from("pachirisu")];
        let attacker = String::from("lapras");

        let non_verbose = program
            .run_match(defenders.clone(), attacker.clone(), false, false)
            .await
            .unwrap();

        let stab_only = program
            .run_match(defenders.clone(), attacker.clone(), false, true)
            .await
            .unwrap();

        let verbose = program
            .run_match(defenders.clone(), attacker.clone(), true, false)
            .await
            .unwrap();

        insta::with_settings!({
            description => "match golem pachirisu lapras --game x-y",
            omit_expression => true
        }, {
            insta::assert_snapshot!(non_verbose);
        });

        insta::with_settings!({
            description => "match golem pachirisu lapras --stab-only --game x-y",
            omit_expression => true
        }, {
            insta::assert_snapshot!(stab_only);
        });

        insta::with_settings!({
            description => "match golem pachirisu lapras --verbose --game x-y",
            omit_expression => true
        }, {
            insta::assert_snapshot!(verbose);
        });
    }

    #[tokio::test]
    async fn run_coverage() {
        let program = setup_program("the-indigo-disk").await;
        let team = vec![
            String::from("flamigo"),
            String::from("cramorant"),
            String::from("ribombee"),
            String::from("ogerpon-cornerstone-mask"),
            String::from("dudunsparce"),
            String::from("sinistcha"),
        ];

        let output = program.run_coverage(team).await.unwrap();

        insta::with_settings!({
            description => "coverage flamigo cramorant ribombee ogerpon-cornerstone-mask dudunsparce sinistcha --game the-indigo-disk",
            omit_expression => true
        }, {
            insta::assert_snapshot!(output);
        });
    }
}
