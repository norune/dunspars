use super::display::*;
use super::ResourceArgs;
use crate::api::game_to_gen;
use crate::models::database::{
    AbilityRow, GameRow, MoveRow, PokemonRow, SelectAllNames, TypeRow, Validate,
};
use crate::models::{Ability, FromName, FromNameCustom, Move, Pokemon, Type};
use crate::resource::config::ConfigFile;
use crate::resource::custom::{CustomCollection, CustomFile};
use crate::resource::database::DatabaseFile;
use crate::resource::{Config, YamlFile};

use std::io::Write;

use anyhow::{anyhow, Result};
use indoc::writedoc;
use rusqlite::Connection;

struct AppContext {
    db: Connection,
    custom: CustomCollection,
    config: Config,
}
impl AppContext {
    fn try_new(config: Config) -> Result<Self> {
        let db_file = if let Some(path) = &config.db_path {
            DatabaseFile::new(path.clone())
        } else {
            DatabaseFile::default()
        };
        let db = db_file.connect()?;

        let custom_file = if let Some(path) = &config.custom_path {
            CustomFile::new(path.clone())
        } else {
            CustomFile::default()
        };
        let custom = custom_file.read()?;

        Ok(Self { db, config, custom })
    }

    fn get_generation(&self) -> Result<u8> {
        let game = match &self.config.game {
            Some(game) => Validate::<GameRow>::validate(&self.db, game)?,
            None => self
                .get_latest_game()
                .ok_or(anyhow!("Cannot find the latest game"))?,
        };
        Ok(game_to_gen(&game, &self.db))
    }

    fn get_latest_game(&self) -> Option<String> {
        GameRow::select_all_names(&self.db)
            .unwrap()
            .last()
            .map(|g| g.to_string())
    }
}

pub trait Command {
    async fn run(&self, config: Config, writer: &mut impl Write) -> Result<i32>;
}

pub struct SetupCommand;
impl Command for SetupCommand {
    async fn run(&self, _config: Config, writer: &mut impl Write) -> Result<i32> {
        let file = DatabaseFile::default();
        file.build_db(writer).await?;
        Ok(0)
    }
}

pub struct PokemonCommand {
    pub name: String,
    pub moves: bool,
    pub evolution: bool,
}
impl Command for PokemonCommand {
    async fn run(&self, config: Config, writer: &mut impl Write) -> Result<i32> {
        let app = AppContext::try_new(config)?;
        let generation = app.get_generation()?;

        let pokemon = Pokemon::from_name(&self.name, generation, &app.db, &app.custom)?;
        let pokemon_display = DisplayComponent::new(&pokemon, app.config.color_enabled);

        let defense_chart = pokemon.get_defense_chart(&app.db)?;
        let defense_chart_ctx = TypeChartComponent {
            type_chart: &defense_chart,
        };
        let type_chart_display = DisplayComponent::new(defense_chart_ctx, app.config.color_enabled);

        writedoc! {
            writer,
            "
            {pokemon_display}

            {type_chart_display}
            "
        }?;

        if self.evolution {
            let evolution_step = pokemon.get_evolution_steps(&app.db)?;
            let evolution_step_display =
                DisplayComponent::new(&evolution_step, app.config.color_enabled);
            writedoc! {
                writer,
                "

                {evolution_step_display}
                "
            }?;
        }

        if self.moves {
            let moves = pokemon.get_learnable_move_list(&app.db)?;
            let move_list_context = MoveListComponent {
                move_list: &moves,
                pokemon: &pokemon,
            };
            let move_list_display =
                DisplayComponent::new(move_list_context, app.config.color_enabled);

            writedoc! {
                writer,
                "

                {move_list_display}
                "
            }?;
        }

        Ok(0)
    }
}

pub struct TypeCommand {
    pub primary_type: String,
    pub secondary_type: Option<String>,
}
impl Command for TypeCommand {
    async fn run(&self, config: Config, writer: &mut impl Write) -> Result<i32> {
        let app = AppContext::try_new(config)?;
        let generation = app.get_generation()?;

        let primary_type = Type::from_name(&self.primary_type, generation, &app.db)?;
        let primary_offense_ctx = TypeChartComponent {
            type_chart: &primary_type.offense_chart,
        };
        let primary_offense_display =
            DisplayComponent::new(primary_offense_ctx, app.config.color_enabled);

        let secondary_type = self
            .secondary_type
            .as_ref()
            .map(|t| Type::from_name(t, generation, &app.db));

        match secondary_type {
            Some(secondary_type) => {
                let secondary_type = secondary_type?;
                let secondary_offense_ctx = TypeChartComponent {
                    type_chart: &secondary_type.offense_chart,
                };
                let secondary_offense_display =
                    DisplayComponent::new(secondary_offense_ctx, app.config.color_enabled);

                let combined_defense = primary_type.defense_chart + secondary_type.defense_chart;
                let defense_ctx = TypeChartComponent {
                    type_chart: &combined_defense,
                };
                let defense_display = DisplayComponent::new(defense_ctx, app.config.color_enabled);

                writedoc! {
                    writer,
                    "
                    {primary_offense_display}

                    {secondary_offense_display}

                    {defense_display}
                    "
                }?;
            }
            None => {
                let defense_ctx = TypeChartComponent {
                    type_chart: &primary_type.defense_chart,
                };
                let defense_display = DisplayComponent::new(defense_ctx, app.config.color_enabled);

                writedoc! {
                    writer,
                    "
                    {primary_offense_display}

                    {defense_display}
                    "
                }?;
            }
        }

        Ok(0)
    }
}

pub struct MoveCommand {
    pub name: String,
}
impl Command for MoveCommand {
    async fn run(&self, config: Config, writer: &mut impl Write) -> Result<i32> {
        let app = AppContext::try_new(config)?;
        let generation = app.get_generation()?;

        let move_ = Move::from_name(&self.name, generation, &app.db)?;
        let move_display = DisplayComponent::new(&move_, app.config.color_enabled);

        writedoc! {
            writer,
            "
            {move_display}
            "
        }?;

        Ok(0)
    }
}

pub struct AbilityCommand {
    pub name: String,
}
impl Command for AbilityCommand {
    async fn run(&self, config: Config, writer: &mut impl Write) -> Result<i32> {
        let app = AppContext::try_new(config)?;
        let generation = app.get_generation()?;

        let ability = Ability::from_name(&self.name, generation, &app.db)?;
        let ability_display = DisplayComponent::new(&ability, app.config.color_enabled);

        writedoc! {
            writer,
            "
            {ability_display}
            "
        }?;

        Ok(0)
    }
}

#[derive(Clone)]
pub struct MatchCommand {
    pub defender_names: Vec<String>,
    pub attacker_name: String,
    pub verbose: bool,
    pub stab_only: bool,
}
impl Command for MatchCommand {
    async fn run(&self, config: Config, writer: &mut impl Write) -> Result<i32> {
        let app = AppContext::try_new(config)?;
        let generation = app.get_generation()?;

        let attacker = Pokemon::from_name(&self.attacker_name, generation, &app.db, &app.custom)?;

        let mut defenders = vec![];

        for defender_name in self.defender_names.iter() {
            let defender = Pokemon::from_name(defender_name, generation, &app.db, &app.custom)?;

            defenders.push(defender);
        }

        for defender in defenders {
            let match_context = MatchComponent {
                defender: &defender,
                attacker: &attacker,
                db: &app.db,
                verbose: self.verbose,
                stab_only: self.stab_only,
            };
            let match_display = DisplayComponent::new(match_context, app.config.color_enabled);

            writedoc! {
                writer,
                "
                {match_display}


                "
            }?;
        }

        Ok(0)
    }
}

pub struct CoverageCommand {
    pub names: Vec<String>,
}
impl Command for CoverageCommand {
    async fn run(&self, config: Config, writer: &mut impl Write) -> Result<i32> {
        let app = AppContext::try_new(config)?;
        let generation = app.get_generation()?;

        let mut pokemon = vec![];
        for name in self.names.iter() {
            let mon = Pokemon::from_name(name, generation, &app.db, &app.custom)?;
            pokemon.push(mon);
        }

        let coverage_ctx = CoverageComponent {
            pokemon: &pokemon,
            db: &app.db,
        };
        let coverage_display = DisplayComponent::new(coverage_ctx, app.config.color_enabled);

        writedoc! {
            writer,
            "
            {coverage_display}
            "
        }?;

        Ok(0)
    }
}

pub struct ResourceCommand {
    pub resource: ResourceArgs,
    pub delimiter: Option<String>,
}
impl Command for ResourceCommand {
    async fn run(&self, config: Config, writer: &mut impl Write) -> Result<i32> {
        let app = AppContext::try_new(config)?;
        let delimiter = self.delimiter.clone().unwrap_or("\n".to_string());

        let resource = match self.resource {
            ResourceArgs::Pokemon => PokemonRow::select_all_names(&app.db)?.join(&delimiter),
            ResourceArgs::Moves => MoveRow::select_all_names(&app.db)?.join(&delimiter),
            ResourceArgs::Abilities => AbilityRow::select_all_names(&app.db)?.join(&delimiter),
            ResourceArgs::Types => TypeRow::select_all_names(&app.db)?.join(&delimiter),
            ResourceArgs::Games => GameRow::select_all_names(&app.db)?.join(&delimiter),
        };

        writedoc! {
            writer,
            "
            {resource}
            "
        }?;

        Ok(0)
    }
}

pub struct ConfigCommand {
    pub key: Option<String>,
    pub value: Option<String>,
    pub unset: bool,
}
impl Command for ConfigCommand {
    async fn run(&self, config: Config, writer: &mut impl Write) -> Result<i32> {
        let config_file = if let Some(path) = config.config_path {
            ConfigFile::new(path)
        } else {
            ConfigFile::default()
        };

        let mut config = config_file.read()?;

        if let Some(key) = &self.key {
            if self.unset {
                config.unset_value(key);
                config_file.save(config)?;
            } else if let Some(value) = &self.value {
                config.set_value(key, value);
                config_file.save(config)?;
            } else if self.value.is_none() {
                if let Some(value) = config.get_value(key) {
                    writeln!(writer, "{value}")?;
                }
            }
        } else {
            for (key, value) in config.get_collection() {
                writeln!(writer, "{key}: {value}")?;
            }
        }

        Ok(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resource::ConfigBuilder;

    fn config(game: &str) -> Config {
        ConfigBuilder::default()
            .game(String::from(game))
            .color_enabled(false)
            .build()
            .unwrap()
    }

    async fn run_command(command: impl Command, config: Config) -> String {
        let mut writer = vec![];
        command.run(config, &mut writer).await.unwrap();
        String::from_utf8(writer).unwrap()
    }

    #[tokio::test]
    async fn run_pokemon() {
        let config = config("scarlet-violet");
        let pokemon = PokemonCommand {
            name: String::from("ceruledge"),
            moves: false,
            evolution: false,
        };

        let output = run_command(pokemon, config).await;

        insta::with_settings!({
            description => "pokemon ceruledge --game scarlet-violet",
            omit_expression => true
        }, {
            insta::assert_snapshot!(output);
        });
    }

    #[tokio::test]
    async fn run_pokemon_evolution() {
        let config = config("sword-shield");
        let cascoon = PokemonCommand {
            name: String::from("cascoon"),
            moves: false,
            evolution: true,
        };
        let cascoon_output = run_command(cascoon, config.clone()).await;

        insta::with_settings!({
            description => "pokemon cascoon --evolution --game sword-shield",
            omit_expression => true
        }, {
            insta::assert_snapshot!(cascoon_output);
        });

        let politoed = PokemonCommand {
            name: String::from("politoed"),
            moves: false,
            evolution: true,
        };
        let politoed_output = run_command(politoed, config.clone()).await;

        insta::with_settings!({
            description => "pokemon politoed --evolution --game sword-shield",
            omit_expression => true
        }, {
            insta::assert_snapshot!(politoed_output);
        });

        let applin = PokemonCommand {
            name: String::from("applin"),
            moves: false,
            evolution: true,
        };
        let applin_output = run_command(applin, config.clone()).await;

        insta::with_settings!({
            description => "pokemon applin --evolution --game sword-shield",
            omit_expression => true
        }, {
            insta::assert_snapshot!(applin_output);
        });
    }

    #[tokio::test]
    async fn run_pokemon_moves() {
        let config = config("scarlet-violet");
        let blaziken = PokemonCommand {
            name: String::from("blaziken"),
            moves: true,
            evolution: false,
        };
        let output = run_command(blaziken, config).await;

        insta::with_settings!({
            description => "pokemon blaziken --moves --game scarlet-violet",
            omit_expression => true
        }, {
            insta::assert_snapshot!(output);
        });
    }

    #[tokio::test]
    async fn run_type() {
        let config = config("platinum");
        let ice = TypeCommand {
            primary_type: String::from("ice"),
            secondary_type: None,
        };
        let output = run_command(ice, config.clone()).await;

        insta::with_settings!({
            description => "type ice --game platinum",
            omit_expression => true
        }, {
            insta::assert_snapshot!(output);
        });

        let ground_water = TypeCommand {
            primary_type: String::from("ground"),
            secondary_type: Some(String::from("water")),
        };
        let output = run_command(ground_water, config.clone()).await;

        insta::with_settings!({
            description => "type ground water --game platinum",
            omit_expression => true
        }, {
            insta::assert_snapshot!(output);
        });
    }

    #[tokio::test]
    async fn run_move() {
        let config = config("sun-moon");
        let brick_break = MoveCommand {
            name: String::from("brick-break"),
        };
        let output = run_command(brick_break, config).await;

        insta::with_settings!({
            description => "move brick-break --game sun-moon",
            omit_expression => true
        }, {
            insta::assert_snapshot!(output);
        });
    }

    #[tokio::test]
    async fn run_ability() {
        let config = config("black-white");
        let intimidate = AbilityCommand {
            name: String::from("intimidate"),
        };
        let output = run_command(intimidate, config).await;

        insta::with_settings!({
            description => "ability intimidate --game black-white",
            omit_expression => true
        }, {
            insta::assert_snapshot!(output);
        });
    }

    #[tokio::test]
    async fn run_match() {
        let config = config("x-y");
        let non_verbose_cmd = MatchCommand {
            defender_names: vec![String::from("golem"), String::from("pachirisu")],
            attacker_name: String::from("lapras"),
            verbose: false,
            stab_only: false,
        };
        let stab_only_cmd = MatchCommand {
            stab_only: true,
            ..non_verbose_cmd.clone()
        };
        let verbose_cmd = MatchCommand {
            verbose: true,
            ..non_verbose_cmd.clone()
        };

        let non_verbose = run_command(non_verbose_cmd, config.clone()).await;
        let stab_only = run_command(stab_only_cmd, config.clone()).await;
        let verbose = run_command(verbose_cmd, config.clone()).await;

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
        let config = config("the-indigo-disk");
        let coverage = CoverageCommand {
            names: vec![
                String::from("flamigo"),
                String::from("cramorant"),
                String::from("ribombee"),
                String::from("ogerpon-cornerstone-mask"),
                String::from("dudunsparce"),
                String::from("sinistcha"),
            ],
        };

        let output = run_command(coverage, config).await;

        insta::with_settings!({
            description => "coverage flamigo cramorant ribombee ogerpon-cornerstone-mask dudunsparce sinistcha --game the-indigo-disk",
            omit_expression => true
        }, {
            insta::assert_snapshot!(output);
        });
    }
}
