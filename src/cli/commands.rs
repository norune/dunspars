use super::display::*;
use super::ResourceArgs;
use crate::api::game_to_gen;
use crate::models::resource::{AbilityRow, GameRow, MoveRow, PokemonRow, Resource, TypeRow};
use crate::models::{Ability, Move, Pokemon, PokemonData, Type};
use crate::resource::config::{Config, ConfigFile};
use crate::resource::database::DatabaseFile;

use std::io::Write;

use anyhow::{anyhow, Result};
use indoc::writedoc;
use rusqlite::Connection;

struct DbContext {
    db: Connection,
    config: Config,
}
impl DbContext {
    fn try_new(config: Config) -> Result<Self> {
        let file = DatabaseFile::default();
        let db = file.connect()?;

        Ok(Self { db, config })
    }

    fn get_generation(&self) -> Result<u8> {
        let game = match &self.config.game {
            Some(game) => self.validate::<GameRow>(game)?,
            None => self
                .get_latest_game()
                .ok_or(anyhow!("Cannot find the latest game"))?,
        };
        Ok(game_to_gen(&game, &self.db))
    }

    fn validate<T: Resource>(&self, name: &str) -> Result<String> {
        T::validate(name, &self.db)
    }

    fn get_latest_game(&self) -> Option<String> {
        GameRow::resource(&self.db).last().map(|g| g.to_string())
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
        let ctx = DbContext::try_new(config)?;
        let generation = ctx.get_generation()?;

        let pokemon_name = ctx.validate::<PokemonRow>(&self.name)?;
        let pokemon = PokemonData::from_name(&pokemon_name, generation, &ctx.db)?;
        let pokemon_display = DisplayComponent::new(&pokemon, ctx.config.color_enabled);

        let defense_chart = pokemon.get_defense_chart(&ctx.db)?;
        let defense_chart_ctx = TypeChartComponent {
            type_chart: &defense_chart,
        };
        let type_chart_display = DisplayComponent::new(defense_chart_ctx, ctx.config.color_enabled);

        writedoc! {
            writer,
            "
            {pokemon_display}

            {type_chart_display}
            "
        }?;

        if self.evolution {
            let evolution_step = pokemon.get_evolution_steps(&ctx.db)?;
            let evolution_step_display =
                DisplayComponent::new(&evolution_step, ctx.config.color_enabled);
            writedoc! {
                writer,
                "

                {evolution_step_display}
                "
            }?;
        }

        if self.moves {
            let moves = pokemon.get_moves(&ctx.db)?;
            let move_list_context = MoveListComponent {
                move_list: &moves,
                pokemon: &pokemon,
            };
            let move_list_display =
                DisplayComponent::new(move_list_context, ctx.config.color_enabled);

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
impl TypeCommand {
    fn get_type(type_name: &str, generation: u8, ctx: &DbContext) -> Result<Type> {
        let type_name = ctx.validate::<TypeRow>(type_name)?;
        let type_ = Type::from_name(&type_name, generation, &ctx.db)?;
        Ok(type_)
    }
}
impl Command for TypeCommand {
    async fn run(&self, config: Config, writer: &mut impl Write) -> Result<i32> {
        let ctx = DbContext::try_new(config)?;
        let generation = ctx.get_generation()?;

        let primary_type = Self::get_type(&self.primary_type, generation, &ctx)?;
        let primary_offense_ctx = TypeChartComponent {
            type_chart: &primary_type.offense_chart,
        };
        let primary_offense_display =
            DisplayComponent::new(primary_offense_ctx, ctx.config.color_enabled);

        let secondary_type = self
            .secondary_type
            .as_ref()
            .map(|t| Self::get_type(t, generation, &ctx));

        match secondary_type {
            Some(secondary_type) => {
                let secondary_type = secondary_type?;
                let secondary_offense_ctx = TypeChartComponent {
                    type_chart: &secondary_type.offense_chart,
                };
                let secondary_offense_display =
                    DisplayComponent::new(secondary_offense_ctx, ctx.config.color_enabled);

                let combined_defense = primary_type.defense_chart + secondary_type.defense_chart;
                let defense_ctx = TypeChartComponent {
                    type_chart: &combined_defense,
                };
                let defense_display = DisplayComponent::new(defense_ctx, ctx.config.color_enabled);

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
                let defense_display = DisplayComponent::new(defense_ctx, ctx.config.color_enabled);

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
        let ctx = DbContext::try_new(config)?;
        let generation = ctx.get_generation()?;

        let move_name = ctx.validate::<MoveRow>(&self.name)?;
        let move_ = Move::from_name(&move_name, generation, &ctx.db)?;
        let move_display = DisplayComponent::new(&move_, ctx.config.color_enabled);

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
        let ctx = DbContext::try_new(config)?;
        let generation = ctx.get_generation()?;

        let ability_name = ctx.validate::<AbilityRow>(&self.name)?;
        let ability = Ability::from_name(&ability_name, generation, &ctx.db)?;
        let ability_display = DisplayComponent::new(&ability, ctx.config.color_enabled);

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
        let ctx = DbContext::try_new(config)?;
        let generation = ctx.get_generation()?;

        let attacker_name = ctx.validate::<PokemonRow>(&self.attacker_name)?;
        let attacker_data = PokemonData::from_name(&attacker_name, generation, &ctx.db)?;
        let attacker_moves = attacker_data.get_moves(&ctx.db)?;
        let attacker_chart = attacker_data.get_defense_chart(&ctx.db)?;
        let attacker = Pokemon::new(attacker_data, attacker_chart, attacker_moves);

        let mut defenders = vec![];

        for defender_name in self.defender_names.iter() {
            let defender_name = ctx.validate::<PokemonRow>(defender_name)?;
            let defender_data = PokemonData::from_name(&defender_name, generation, &ctx.db)?;
            let defender_moves = defender_data.get_moves(&ctx.db)?;
            let defender_chart = defender_data.get_defense_chart(&ctx.db)?;
            let defender = Pokemon::new(defender_data, defender_chart, defender_moves);

            defenders.push(defender);
        }

        for defender in defenders {
            let match_context = MatchComponent {
                defender: &defender,
                attacker: &attacker,
                verbose: self.verbose,
                stab_only: self.stab_only,
            };
            let match_display = DisplayComponent::new(match_context, ctx.config.color_enabled);

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
        let ctx = DbContext::try_new(config)?;
        let generation = ctx.get_generation()?;

        let mut pokemon = vec![];
        for name in self.names.iter() {
            let name = ctx.validate::<PokemonRow>(name)?;
            let data = PokemonData::from_name(&name, generation, &ctx.db)?;
            let moves = data.get_moves(&ctx.db)?;
            let chart = data.get_defense_chart(&ctx.db)?;

            let mon = Pokemon::new(data, chart, moves);
            pokemon.push(mon);
        }

        let coverage_ctx = CoverageComponent {
            pokemon: &pokemon,
            db: &ctx.db,
        };
        let coverage_display = DisplayComponent::new(coverage_ctx, ctx.config.color_enabled);

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
        let ctx = DbContext::try_new(config)?;
        let delimiter = self.delimiter.clone().unwrap_or("\n".to_string());

        let resource = match self.resource {
            ResourceArgs::Pokemon => PokemonRow::resource(&ctx.db).join(&delimiter),
            ResourceArgs::Moves => MoveRow::resource(&ctx.db).join(&delimiter),
            ResourceArgs::Abilities => AbilityRow::resource(&ctx.db).join(&delimiter),
            ResourceArgs::Types => TypeRow::resource(&ctx.db).join(&delimiter),
            ResourceArgs::Games => GameRow::resource(&ctx.db).join(&delimiter),
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
    pub key: String,
    pub value: Option<String>,
    pub unset: bool,
}
impl Command for ConfigCommand {
    async fn run(&self, _config: Config, writer: &mut impl Write) -> Result<i32> {
        let mut config_file = ConfigFile::from_file()?;
        if self.unset {
            config_file.unset_value(&self.key);
            config_file.save()?;
        } else if let Some(value) = &self.value {
            config_file.set_value(&self.key, value);
            config_file.save()?;
        } else if self.value.is_none() {
            if let Some(value) = config_file.get_value(&self.key) {
                writeln!(writer, "{value}")?;
            }
        }
        Ok(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resource::config::ConfigBuilder;

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
