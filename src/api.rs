use std::collections::HashMap;

use anyhow::{anyhow, bail, Result};
use dirs;
use futures::stream::FuturesUnordered;
use futures::StreamExt;
use regex::Regex;

use rustemon::client::{
    CACacheManager as RustemonCacheManager, CacheMode as RustemonCacheMode, RustemonClient,
    RustemonClientBuilder,
};
use rustemon::games::version_group as rustemon_version;
use rustemon::moves::move_ as rustemon_moves;
use rustemon::pokemon::pokemon_species as rustemon_species;
use rustemon::pokemon::{
    ability as rustemon_ability, pokemon as rustemon_pokemon, type_ as rustemon_type,
};
use rustemon::Follow;

use rustemon::model::evolution::{
    ChainLink as RustemonEvoStep, EvolutionChain as RustemonEvoRoot,
    EvolutionDetail as RustemonEvoMethod,
};
use rustemon::model::moves::{Move as RustemonMove, PastMoveStatValues as RustemonPastMoveStats};
use rustemon::model::pokemon::{
    Ability as RustemonAbility, AbilityEffectChange as RustemonPastAbilityEffect,
    Pokemon as RustemonPokemon, PokemonAbility as RustemonPokemonAbility,
    PokemonMove as RustemonPokemonMove, PokemonSpecies as RustemonSpecies,
    PokemonStat as RustemonStat, PokemonType as RustemonTypeSlot,
    PokemonTypePast as RustemonPastPokemonType, Type as RustemonType,
    TypeRelations as RustemonTypeRelations, TypeRelationsPast as RustemonPastTypeRelations,
};
use rustemon::model::resource::{Effect as RustemonEffect, VerboseEffect as RustemonVerboseEffect};

use crate::pokemon::{
    Ability, EvolutionMethod, EvolutionStep, Move, PokemonData, PokemonGroup, Stats, Type,
    TypeChart,
};

#[derive(Debug)]
pub struct ApiWrapper {
    pub client: RustemonClient,
    pub gen_map: GenerationMap,
    pub gen_regex: Regex,
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

        // PokéAPI keeps generation names in Roman numerals.
        // Might be quicker to just take it from resource urls via regex instead.
        // Regex compilation is expensive, so we're compiling it just once here.
        let gen_regex = Regex::new(r"generation/(?P<gen>\d+)/?$").unwrap();
        let gen_map = GenerationMap::try_new(&client, &gen_regex).await?;

        Ok(Self {
            client,
            gen_map,
            gen_regex,
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

        let current_generation = get_gen_from_game(game, &self.gen_map);
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
            stats: extract_stats(stats),
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
        let pokemon_types = self.match_past(generation, past_types).unwrap_or(types);

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
                let vg_gen = get_gen_from_game(&vg.version_group.name, &self.gen_map);
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

        let relations = self
            .match_past(current_generation, past_damage_relations)
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

        if let Some(past_stats) = self.match_past(current_generation, past_values) {
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

        if let Some(past_effects) = self.match_past(current_generation, effect_changes) {
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

        if let Some(past_effects) = self.match_past(current_generation, effect_changes) {
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
        let evolution_step = traverse_chain(chain);

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
        let generation = get_gen_from_url(url, &self.gen_regex);
        if current_generation < generation {
            bail!(format!(
                "{resource} '{label}' is not present in generation {current_generation}"
            ))
        }
        Ok(())
    }

    fn match_past<T: Past<U>, U>(&self, current_generation: u8, pasts: Vec<T>) -> Option<U> {
        let mut oldest_value = None;
        let mut oldest_generation = 255;

        for past in pasts {
            let past_generation = past.generation(self);
            if current_generation <= past_generation && past_generation <= oldest_generation {
                oldest_value = Some(past.value());
                oldest_generation = past_generation;
            }
        }

        oldest_value
    }
}

#[derive(Debug)]
pub struct GenerationMap(HashMap<String, u8>);
impl GenerationMap {
    pub async fn try_new(client: &RustemonClient, gen_regex: &Regex) -> Result<Self> {
        let mut gen_map = HashMap::new();
        let game_names = get_all_games(client).await?;
        let game_data_futures: FuturesUnordered<_> = game_names
            .iter()
            .map(|g| rustemon_version::get_by_name(g, client))
            .collect();
        let game_data: Vec<_> = game_data_futures.collect().await;

        for game in game_data {
            let game = game?;
            let generation = get_gen_from_url(&game.generation.url, gen_regex);
            gen_map.insert(game.name, generation);
        }

        Ok(GenerationMap(gen_map))
    }

    fn get_generation(&self, game: &str) -> u8 {
        *self.0.get(game).unwrap()
    }
}

pub async fn get_all_pokemon(client: &RustemonClient) -> Result<Vec<String>> {
    Ok(rustemon_pokemon::get_all_entries(client)
        .await?
        .into_iter()
        .map(|p| p.name)
        .collect::<Vec<String>>())
}

pub async fn get_all_types(client: &RustemonClient) -> Result<Vec<String>> {
    Ok(rustemon_type::get_all_entries(client)
        .await?
        .into_iter()
        .map(|p| p.name)
        .collect::<Vec<String>>())
}

pub async fn get_all_moves(client: &RustemonClient) -> Result<Vec<String>> {
    Ok(rustemon_moves::get_all_entries(client)
        .await?
        .into_iter()
        .map(|p| p.name)
        .collect::<Vec<String>>())
}

pub async fn get_all_abilities(client: &RustemonClient) -> Result<Vec<String>> {
    Ok(rustemon_ability::get_all_entries(client)
        .await?
        .into_iter()
        .map(|p| p.name)
        .collect::<Vec<String>>())
}

pub async fn get_all_games(client: &RustemonClient) -> Result<Vec<String>> {
    Ok(rustemon_version::get_all_entries(client)
        .await?
        .into_iter()
        .map(|p| p.name)
        .collect::<Vec<String>>())
}

pub fn get_gen_from_game(game: &str, gen_map: &GenerationMap) -> u8 {
    gen_map.get_generation(game)
}

pub fn get_gen_from_url(url: &str, regex: &Regex) -> u8 {
    let caps = regex.captures(url).unwrap();
    caps["gen"].parse::<u8>().unwrap()
}

fn traverse_chain(chain_link: RustemonEvoStep) -> EvolutionStep {
    let evolution_methods = chain_link
        .evolution_details
        .into_iter()
        .map(convert_to_evolution_method)
        .collect();

    if !chain_link.evolves_to.is_empty() {
        let evolves_to = chain_link
            .evolves_to
            .into_iter()
            .map(traverse_chain)
            .collect();

        EvolutionStep::new(chain_link.species.name, evolution_methods, evolves_to)
    } else {
        EvolutionStep::new(chain_link.species.name, evolution_methods, vec![])
    }
}

fn convert_to_evolution_method(evolution: RustemonEvoMethod) -> EvolutionMethod {
    let mut method = EvolutionMethod::new(evolution.trigger.name);
    if let Some(item) = evolution.item {
        method = method.item(item.name);
    }
    if let Some(gender) = evolution.gender {
        method = method.gender(gender);
    }
    if let Some(held_item) = evolution.held_item {
        method = method.held_item(held_item.name);
    }
    if let Some(known_move) = evolution.known_move {
        method = method.known_move(known_move.name);
    }
    if let Some(known_move_type) = evolution.known_move_type {
        method = method.known_move_type(known_move_type.name);
    }
    if let Some(location) = evolution.location {
        method = method.location(location.name);
    }
    if let Some(min_level) = evolution.min_level {
        method = method.min_level(min_level);
    }
    if let Some(min_happiness) = evolution.min_happiness {
        method = method.min_happiness(min_happiness);
    }
    if let Some(min_beauty) = evolution.min_beauty {
        method = method.min_beauty(min_beauty);
    }
    if let Some(min_affection) = evolution.min_affection {
        method = method.min_affection(min_affection);
    }
    if let Some(party_species) = evolution.party_species {
        method = method.party_species(party_species.name);
    }
    if let Some(party_type) = evolution.party_type {
        method = method.party_type(party_type.name);
    }
    if let Some(relative_physical_stats) = evolution.relative_physical_stats {
        method = method.relative_physical_stats(relative_physical_stats);
    }
    if let Some(trade_species) = evolution.trade_species {
        method = method.trade_species(trade_species.name);
    }
    if evolution.needs_overworld_rain {
        method = method.needs_overworld_rain(true);
    }
    if evolution.turn_upside_down {
        method = method.turn_upside_down(true);
    }
    if !evolution.time_of_day.is_empty() {
        method = method.time_of_day(evolution.time_of_day);
    }

    method
}

fn extract_stats(stats_vec: Vec<RustemonStat>) -> Stats {
    let mut stats = Stats::default();

    for RustemonStat {
        stat, base_stat, ..
    } in stats_vec
    {
        match stat.name.as_str() {
            "hp" => stats.hp = base_stat,
            "attack" => stats.attack = base_stat,
            "defense" => stats.defense = base_stat,
            "special-attack" => stats.special_attack = base_stat,
            "special-defense" => stats.special_defense = base_stat,
            "speed" => stats.speed = base_stat,
            _ => (),
        }
    }

    stats
}

trait Past<T> {
    fn generation(&self, api: &ApiWrapper) -> u8;
    fn value(self) -> T;
}

impl Past<Vec<RustemonTypeSlot>> for RustemonPastPokemonType {
    fn generation(&self, api: &ApiWrapper) -> u8 {
        get_gen_from_url(&self.generation.url, &api.gen_regex)
    }

    fn value(self) -> Vec<RustemonTypeSlot> {
        self.types
    }
}

impl Past<RustemonTypeRelations> for RustemonPastTypeRelations {
    fn generation(&self, api: &ApiWrapper) -> u8 {
        get_gen_from_url(&self.generation.url, &api.gen_regex)
    }

    fn value(self) -> RustemonTypeRelations {
        self.damage_relations
    }
}

impl Past<RustemonPastMoveStats> for RustemonPastMoveStats {
    fn generation(&self, api: &ApiWrapper) -> u8 {
        get_gen_from_game(&self.version_group.name, &api.gen_map) - 1
    }

    fn value(self) -> RustemonPastMoveStats {
        self
    }
}

impl Past<Vec<RustemonEffect>> for RustemonPastAbilityEffect {
    fn generation(&self, api: &ApiWrapper) -> u8 {
        get_gen_from_game(&self.version_group.name, &api.gen_map) - 1
    }

    fn value(self) -> Vec<RustemonEffect> {
        self.effect_entries
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_type() {
        let api = ApiWrapper::try_new().await.unwrap();

        // Fairy was not introduced until gen 6
        api.get_type("fairy", 3).await.unwrap_err();
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
    async fn test_move() {
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
    async fn test_ability() {
        let api = ApiWrapper::try_new().await.unwrap();

        // Beads of Ruin was not introduced until gen 9
        api.get_ability("beads-of-ruin", 8).await.unwrap_err();
        api.get_ability("beads-of-ruin", 9).await.unwrap();
    }

    #[tokio::test]
    async fn test_pokemon() {
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
