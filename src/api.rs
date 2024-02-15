mod utils;
use utils::GenerationResource;

use std::collections::HashMap;

use anyhow::{anyhow, bail, Result};
use dirs;
use rustemon::client::{
    CACacheManager as RustemonCacheManager, CacheMode as RustemonCacheMode, RustemonClient,
    RustemonClientBuilder,
};
use rustemon::moves::move_ as rustemon_moves;
use rustemon::pokemon::pokemon_species as rustemon_species;
use rustemon::pokemon::{
    ability as rustemon_ability, pokemon as rustemon_pokemon, type_ as rustemon_type,
};
use rustemon::Follow;

use rustemon::model::evolution::EvolutionChain as RustemonEvoRoot;
use rustemon::model::moves::Move as RustemonMove;
use rustemon::model::pokemon::{
    Ability as RustemonAbility, Pokemon as RustemonPokemon,
    PokemonAbility as RustemonPokemonAbility, PokemonMove as RustemonPokemonMove,
    PokemonSpecies as RustemonSpecies, PokemonType as RustemonTypeSlot,
    PokemonTypePast as RustemonPastPokemonType, Type as RustemonType,
};
use rustemon::model::resource::VerboseEffect as RustemonVerboseEffect;

use crate::pokemon::{Ability, EvolutionStep, Move, PokemonData, PokemonGroup, Type, TypeChart};

#[derive(Debug)]
pub struct ApiWrapper {
    pub client: RustemonClient,
    pub generation_resource: GenerationResource,
    cache_manager: RustemonCacheManager,
}

impl ApiWrapper {
    pub async fn try_new() -> Result<Self> {
        let mut cache_dir = if let Some(home_dir) = dirs::home_dir() {
            home_dir
        } else {
            bail!("Home directory not found")
        };
        cache_dir.push(".cache/dunspars/rustemon");

        let cache_manager = RustemonCacheManager { path: cache_dir };
        // This disregards cache staleness. Pokémon data is not likely to change
        // Cache should be cleared by user via program command
        let cache_mode = RustemonCacheMode::ForceCache;
        let client = RustemonClientBuilder::default()
            .with_manager(cache_manager.clone())
            .with_mode(cache_mode)
            .try_build()?;

        let generation_resource = GenerationResource::try_new(&client).await?;

        Ok(Self {
            client,
            generation_resource,
            cache_manager,
        })
    }

    pub async fn get_pokemon(&self, pokemon: &str, game: &str) -> Result<PokemonData> {
        let RustemonPokemon {
            name,
            types,
            past_types,
            moves,
            stats,
            abilities,
            species,
            ..
        } = rustemon_pokemon::get_by_name(pokemon, &self.client).await?;

        let current_generation = self.generation_resource.get_gen_from_game(game);
        let learn_moves = self.get_pokemon_moves(moves, current_generation);
        // PokéAPI doesn't seem to supply a field that denotes when a Pokémon was introduced.
        // So the next best thing is to check if they have any moves in the specified generation.
        if learn_moves.is_empty() {
            bail!(format!(
                "Pokémon '{pokemon}' is not present in generation {current_generation}"
            ))
        }

        let (primary_type, secondary_type) =
            self.get_pokemon_type(types, past_types, current_generation);
        let abilities = self.get_pokemon_abilities(abilities);

        let group = self.get_pokemon_group(&species.name).await?;

        Ok(PokemonData {
            name,
            primary_type,
            secondary_type,
            learn_moves,
            abilities,
            species: species.name,
            group,
            stats: utils::extract_stats(stats),
            game: game.to_string(),
            generation: current_generation,
            api: self,
        })
    }

    fn get_pokemon_type(
        &self,
        types: Vec<RustemonTypeSlot>,
        past_types: Vec<RustemonPastPokemonType>,
        generation: u8,
    ) -> (String, Option<String>) {
        let pokemon_types =
            utils::match_past(generation, past_types, &self.generation_resource).unwrap_or(types);

        let primary_type = pokemon_types
            .iter()
            .find(|t| t.slot == 1)
            .unwrap()
            .type_
            .name
            .clone();
        let secondary_type = pokemon_types
            .iter()
            .find(|t| t.slot == 2)
            .map(|t| t.type_.name.clone());

        (primary_type, secondary_type)
    }

    fn get_pokemon_moves(
        &self,
        moves: Vec<RustemonPokemonMove>,
        generation: u8,
    ) -> HashMap<String, (String, i64)> {
        let mut learn_moves = HashMap::new();
        for move_ in moves {
            let learnable_move = move_.version_group_details.iter().find(|vg| {
                let vg_gen = self
                    .generation_resource
                    .get_gen_from_game(&vg.version_group.name);
                vg_gen == generation
            });

            if let Some(learn_move) = learnable_move {
                learn_moves.insert(
                    move_.move_.name.clone(),
                    (
                        learn_move.move_learn_method.name.clone(),
                        learn_move.level_learned_at,
                    ),
                );
            }
        }
        learn_moves
    }

    fn get_pokemon_abilities(&self, abilities: Vec<RustemonPokemonAbility>) -> Vec<(String, bool)> {
        abilities
            .iter()
            .map(|a| (a.ability.name.clone(), a.is_hidden))
            .collect::<Vec<_>>()
    }

    async fn get_pokemon_group(&self, species: &str) -> Result<PokemonGroup> {
        let RustemonSpecies {
            is_legendary,
            is_mythical,
            ..
        } = rustemon_species::get_by_name(species, &self.client).await?;

        if is_mythical {
            return Ok(PokemonGroup::Mythical);
        }

        if is_legendary {
            return Ok(PokemonGroup::Legendary);
        }

        Ok(PokemonGroup::Regular)
    }

    pub async fn get_type(&self, type_str: &str, current_generation: u8) -> Result<Type> {
        let RustemonType {
            name,
            damage_relations,
            past_damage_relations,
            generation,
            ..
        } = rustemon_type::get_by_name(type_str, &self.client).await?;

        self.check_generation("Type", type_str, &generation.url, current_generation)?;

        let relations = utils::match_past(
            current_generation,
            past_damage_relations,
            &self.generation_resource,
        )
        .unwrap_or(damage_relations);

        let mut offense_chart = HashMap::new();
        let mut defense_chart = HashMap::new();

        relations.no_damage_to.iter().for_each(|t| {
            offense_chart.insert(t.name.to_string(), 0.0);
        });
        relations.half_damage_to.iter().for_each(|t| {
            offense_chart.insert(t.name.to_string(), 0.5);
        });
        relations.double_damage_to.iter().for_each(|t| {
            offense_chart.insert(t.name.to_string(), 2.0);
        });

        relations.no_damage_from.iter().for_each(|t| {
            defense_chart.insert(t.name.to_string(), 0.0);
        });
        relations.half_damage_from.iter().for_each(|t| {
            defense_chart.insert(t.name.to_string(), 0.5);
        });
        relations.double_damage_from.iter().for_each(|t| {
            defense_chart.insert(t.name.to_string(), 2.0);
        });

        Ok(Type {
            name,
            offense_chart: TypeChart::new(offense_chart),
            defense_chart: TypeChart::new(defense_chart),
            generation: current_generation,
            api: self,
        })
    }

    pub async fn get_move(&self, name: &str, current_generation: u8) -> Result<Move> {
        let RustemonMove {
            name,
            mut accuracy,
            mut power,
            mut pp,
            damage_class,
            mut type_,
            mut effect_chance,
            effect_entries,
            effect_changes,
            past_values,
            generation,
            ..
        } = rustemon_moves::get_by_name(name, &self.client).await?;

        self.check_generation("Move", &name, &generation.url, current_generation)?;

        let RustemonVerboseEffect {
            mut effect,
            mut short_effect,
            ..
        } = effect_entries
            .into_iter()
            .find(|e| e.language.name == "en")
            .unwrap_or_default();

        if let Some(past_stats) =
            utils::match_past(current_generation, past_values, &self.generation_resource)
        {
            accuracy = past_stats.accuracy.or(accuracy);
            power = past_stats.power.or(power);
            pp = past_stats.pp.or(pp);
            effect_chance = past_stats.effect_chance.or(effect_chance);

            if let Some(t) = past_stats.type_ {
                type_ = t;
            }

            if let Some(entry) = past_stats
                .effect_entries
                .into_iter()
                .find(|e| e.language.name == "en")
            {
                effect = entry.effect;
                short_effect = entry.short_effect;
            }
        }

        if let Some(past_effects) = utils::match_past(
            current_generation,
            effect_changes,
            &self.generation_resource,
        ) {
            if let Some(past_effect) = past_effects.into_iter().find(|e| e.language.name == "en") {
                effect += format!(
                    "\n\nGeneration {current_generation}: {}",
                    past_effect.effect
                )
                .as_str();
            }
        }

        Ok(Move {
            name,
            accuracy,
            power,
            pp,
            damage_class: damage_class.name,
            type_: type_.name,
            effect_chance,
            effect,
            short_effect,
            generation: current_generation,
            api: self,
        })
    }

    pub async fn get_ability(&self, name: &str, current_generation: u8) -> Result<Ability> {
        let RustemonAbility {
            name,
            effect_entries,
            effect_changes,
            generation,
            ..
        } = rustemon_ability::get_by_name(name, &self.client).await?;

        self.check_generation("Ability", &name, &generation.url, current_generation)?;

        let RustemonVerboseEffect {
            mut effect,
            short_effect,
            ..
        } = effect_entries
            .into_iter()
            .find(|e| e.language.name == "en")
            .unwrap_or_default();

        if let Some(past_effects) = utils::match_past(
            current_generation,
            effect_changes,
            &self.generation_resource,
        ) {
            if let Some(past_effect) = past_effects.into_iter().find(|e| e.language.name == "en") {
                effect += format!(
                    "\n\nGeneration {current_generation}: {}",
                    past_effect.effect
                )
                .as_str();
            }
        }

        Ok(Ability {
            name,
            effect,
            short_effect,
            generation: current_generation,
            api: self,
        })
    }

    pub async fn get_evolution_steps(&self, species: &str) -> Result<EvolutionStep> {
        let RustemonEvoRoot { chain, .. } = rustemon_species::get_by_name(species, &self.client)
            .await?
            .evolution_chain
            .unwrap()
            .follow(&self.client)
            .await?;
        let evolution_step = utils::traverse_chain(chain);

        Ok(evolution_step)
    }

    pub async fn clear_cache(&self) -> Result<()> {
        match self.cache_manager.clear().await {
            std::result::Result::Ok(_) => Ok(()),
            std::result::Result::Err(e) => Err(anyhow!(e)),
        }
    }

    fn check_generation(
        &self,
        resource: &'static str,
        label: &str,
        url: &str,
        current_generation: u8,
    ) -> Result<()> {
        let generation = self.generation_resource.get_gen_from_url(url);
        if current_generation < generation {
            bail!(format!(
                "{resource} '{label}' is not present in generation {current_generation}"
            ))
        }
        Ok(())
    }
}

pub enum ResourceResult {
    Valid,
    Invalid(Vec<String>),
}

#[allow(async_fn_in_trait)]
pub trait Resource: Sized {
    fn get_matches(&self, value: &str) -> Vec<String> {
        self.resource()
            .iter()
            .filter_map(|r| {
                let close_enough = if !r.is_empty() && !value.is_empty() {
                    let first_r = r.chars().next().unwrap();
                    let first_value = value.chars().next().unwrap();

                    // Only perform spellcheck on first character match; potentially expensive
                    first_r == first_value && strsim::levenshtein(r, value) < 4
                } else {
                    false
                };

                if r.contains(value) || close_enough {
                    Some(r.clone())
                } else {
                    None
                }
            })
            .collect::<Vec<String>>()
    }

    fn check(&self, value: &str) -> ResourceResult {
        let matches = self.get_matches(value);
        if matches.iter().any(|m| *m == value) {
            ResourceResult::Valid
        } else {
            ResourceResult::Invalid(matches)
        }
    }

    fn validate(&self, value: &str) -> Result<String> {
        let value = value.to_lowercase();
        match self.check(&value) {
            ResourceResult::Valid => Ok(value),
            ResourceResult::Invalid(matches) => bail!(Self::invalid_message(&value, &matches)),
        }
    }

    fn invalid_message(value: &str, matches: &[String]) -> String {
        let resource_name = Self::label();
        let mut message = format!("{resource_name} '{value}' not found.");

        if matches.len() > 20 {
            message += " Potential matches found; too many to display.";
        } else if !matches.is_empty() {
            message += &format!(" Potential matches: {}.", matches.join(" "));
        }

        message
    }

    async fn try_new(api: &ApiWrapper) -> Result<Self>;
    fn resource(&self) -> &Vec<String>;
    fn label() -> &'static str;
}

pub struct PokemonResource {
    resource: Vec<String>,
}
impl Resource for PokemonResource {
    async fn try_new(api: &ApiWrapper) -> Result<Self> {
        let resource = utils::get_all_pokemon(&api.client).await?;
        Ok(Self { resource })
    }

    fn resource(&self) -> &Vec<String> {
        &self.resource
    }

    fn label() -> &'static str {
        "Pokémon"
    }
}

pub struct GameResource {
    resource: Vec<String>,
}
impl Resource for GameResource {
    async fn try_new(api: &ApiWrapper) -> Result<Self> {
        let resource = utils::get_all_games(&api.client).await?;
        Ok(Self { resource })
    }

    fn resource(&self) -> &Vec<String> {
        &self.resource
    }

    fn label() -> &'static str {
        "Game"
    }
}

pub struct TypeResource {
    resource: Vec<String>,
}
impl Resource for TypeResource {
    async fn try_new(api: &ApiWrapper) -> Result<Self> {
        let resource = utils::get_all_types(&api.client).await?;
        Ok(Self { resource })
    }

    fn resource(&self) -> &Vec<String> {
        &self.resource
    }

    fn label() -> &'static str {
        "Type"
    }
}

pub struct MoveResource {
    resource: Vec<String>,
}
impl Resource for MoveResource {
    async fn try_new(api: &ApiWrapper) -> Result<Self> {
        let resource = utils::get_all_moves(&api.client).await?;
        Ok(Self { resource })
    }

    fn resource(&self) -> &Vec<String> {
        &self.resource
    }

    fn label() -> &'static str {
        "Move"
    }
}

pub struct AbilityResource {
    resource: Vec<String>,
}
impl Resource for AbilityResource {
    async fn try_new(api: &ApiWrapper) -> Result<Self> {
        let resource = utils::get_all_abilities(&api.client).await?;
        Ok(Self { resource })
    }

    fn resource(&self) -> &Vec<String> {
        &self.resource
    }

    fn label() -> &'static str {
        "Ability"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn get_type() {
        let api = ApiWrapper::try_new().await.unwrap();

        // Fairy was not introduced until gen 6
        api.get_type("fairy", 5).await.unwrap_err();
        api.get_type("fairy", 6).await.unwrap();

        // Bug gen 1 2x against poison
        let bug_gen_1 = api.get_type("bug", 1).await.unwrap();
        assert_eq!(2.0, bug_gen_1.offense_chart.get_multiplier("poison"));
        assert_eq!(1.0, bug_gen_1.offense_chart.get_multiplier("dark"));

        // Bug gen >=2 2x against dark
        let bug_gen_2 = api.get_type("bug", 2).await.unwrap();
        assert_eq!(0.5, bug_gen_2.offense_chart.get_multiplier("poison"));
        assert_eq!(2.0, bug_gen_2.offense_chart.get_multiplier("dark"));
    }

    #[tokio::test]
    async fn get_move() {
        let api = ApiWrapper::try_new().await.unwrap();

        // Earth Power was not introduced until gen 4
        api.get_move("earth-power", 3).await.unwrap_err();
        api.get_move("earth-power", 4).await.unwrap();

        // Tackle gen 1-4 power: 35 accuracy: 95
        let tackle_gen_4 = api.get_move("tackle", 4).await.unwrap();
        assert_eq!(35, tackle_gen_4.power.unwrap());
        assert_eq!(95, tackle_gen_4.accuracy.unwrap());

        // Tackle gen 5-6 power: 50 accuracy: 100
        let tackle_gen_5 = api.get_move("tackle", 5).await.unwrap();
        assert_eq!(50, tackle_gen_5.power.unwrap());
        assert_eq!(100, tackle_gen_5.accuracy.unwrap());

        // Tackle gen >=7 power: 40 accuracy: 100
        let tackle_gen_9 = api.get_move("tackle", 9).await.unwrap();
        assert_eq!(40, tackle_gen_9.power.unwrap());
        assert_eq!(100, tackle_gen_9.accuracy.unwrap());
    }

    #[tokio::test]
    async fn get_ability() {
        let api = ApiWrapper::try_new().await.unwrap();

        // Beads of Ruin was not introduced until gen 9
        api.get_ability("beads-of-ruin", 8).await.unwrap_err();
        api.get_ability("beads-of-ruin", 9).await.unwrap();
    }

    #[tokio::test]
    async fn get_pokemon() {
        let api = ApiWrapper::try_new().await.unwrap();

        // Ogerpon was not inroduced until gen 9
        api.get_pokemon("ogerpon", "sword-shield")
            .await
            .unwrap_err();
        api.get_pokemon("ogerpon", "the-teal-mask").await.unwrap();

        // Wailord is not present in gen 9, but is present in gen 8
        api.get_pokemon("wailord", "scarlet-violet")
            .await
            .unwrap_err();
        api.get_pokemon("wailord", "sword-shield").await.unwrap();

        // Test dual type defense chart
        let golem = api.get_pokemon("golem", "scarlet-violet").await.unwrap();
        let golem_defense = golem.get_defense_chart().await.unwrap();
        assert_eq!(4.0, golem_defense.get_multiplier("water"));
        assert_eq!(2.0, golem_defense.get_multiplier("fighting"));
        assert_eq!(1.0, golem_defense.get_multiplier("psychic"));
        assert_eq!(0.5, golem_defense.get_multiplier("flying"));
        assert_eq!(0.25, golem_defense.get_multiplier("poison"));
        assert_eq!(0.0, golem_defense.get_multiplier("electric"));

        // Clefairy was Normal type until gen 6
        let clefairy_gen_5 = api.get_pokemon("clefairy", "black-white").await.unwrap();
        assert_eq!("normal", clefairy_gen_5.primary_type);
        let clefairy_gen_6 = api.get_pokemon("clefairy", "x-y").await.unwrap();
        assert_eq!("fairy", clefairy_gen_6.primary_type);
    }
}
