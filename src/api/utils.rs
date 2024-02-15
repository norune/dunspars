use std::collections::HashMap;

use anyhow::{bail, Result};
use futures::stream::FuturesUnordered;
use futures::StreamExt;
use regex::Regex;

use rustemon::client::RustemonClient;
use rustemon::games::version_group as rustemon_version;
use rustemon::moves::move_ as rustemon_moves;
use rustemon::pokemon::{
    ability as rustemon_ability, pokemon as rustemon_pokemon, type_ as rustemon_type,
};

use rustemon::model::evolution::{
    ChainLink as RustemonEvoStep, EvolutionDetail as RustemonEvoMethod,
};
use rustemon::model::moves::PastMoveStatValues as RustemonPastMoveStats;
use rustemon::model::pokemon::{
    AbilityEffectChange as RustemonPastAbilityEffect, PokemonStat as RustemonStat,
    PokemonType as RustemonTypeSlot, PokemonTypePast as RustemonPastPokemonType,
    TypeRelations as RustemonTypeRelations, TypeRelationsPast as RustemonPastTypeRelations,
};
use rustemon::model::resource::Effect as RustemonEffect;

use crate::pokemon::{EvolutionMethod, EvolutionStep, Stats};

#[derive(Debug)]
pub struct GenerationResource {
    gen_url_regex: Regex,
    game_map: HashMap<String, u8>,
}

impl GenerationResource {
    pub async fn try_new(client: &RustemonClient) -> Result<Self> {
        // PokéAPI keeps generation names in Roman numerals.
        // Might be quicker to just take it from resource urls via regex instead.
        // Regex compilation is expensive, so we're compiling it just once here.
        let gen_url_regex = Regex::new(r"generation/(?P<gen>\d+)/?$").unwrap();

        let mut game_map = HashMap::new();
        let game_names = get_all_games(client).await?;
        let game_data_futures: FuturesUnordered<_> = game_names
            .iter()
            .map(|g| rustemon_version::get_by_name(g, client))
            .collect();
        let game_data: Vec<_> = game_data_futures.collect().await;

        for game in game_data {
            let game = game?;
            let generation = capture_gen_url(&game.generation.url, &gen_url_regex).unwrap();
            game_map.insert(game.name, generation);
        }

        Ok(GenerationResource {
            game_map,
            gen_url_regex,
        })
    }

    pub fn get_gen_from_game(&self, game: &str) -> u8 {
        *self.game_map.get(game).unwrap()
    }

    pub fn get_gen_from_url(&self, url: &str) -> u8 {
        capture_gen_url(url, &self.gen_url_regex).unwrap()
    }
}

fn capture_gen_url(url: &str, gen_url_regex: &Regex) -> Result<u8> {
    if let Some(caps) = gen_url_regex.captures(url) {
        Ok(caps["gen"].parse::<u8>()?)
    } else {
        bail!("Generation not found in resource url");
    }
}

pub trait Past<T> {
    fn generation(&self, resource: &GenerationResource) -> u8;
    fn value(self) -> T;
}

impl Past<Vec<RustemonTypeSlot>> for RustemonPastPokemonType {
    fn generation(&self, resource: &GenerationResource) -> u8 {
        resource.get_gen_from_url(&self.generation.url)
    }

    fn value(self) -> Vec<RustemonTypeSlot> {
        self.types
    }
}

impl Past<RustemonTypeRelations> for RustemonPastTypeRelations {
    fn generation(&self, resource: &GenerationResource) -> u8 {
        resource.get_gen_from_url(&self.generation.url)
    }

    fn value(self) -> RustemonTypeRelations {
        self.damage_relations
    }
}

impl Past<RustemonPastMoveStats> for RustemonPastMoveStats {
    fn generation(&self, resource: &GenerationResource) -> u8 {
        resource.get_gen_from_game(&self.version_group.name) - 1
    }

    fn value(self) -> RustemonPastMoveStats {
        self
    }
}

impl Past<Vec<RustemonEffect>> for RustemonPastAbilityEffect {
    fn generation(&self, resource: &GenerationResource) -> u8 {
        resource.get_gen_from_game(&self.version_group.name) - 1
    }

    fn value(self) -> Vec<RustemonEffect> {
        self.effect_entries
    }
}

pub fn match_past<T: Past<U>, U>(
    current_generation: u8,
    pasts: Vec<T>,
    generation_resource: &GenerationResource,
) -> Option<U> {
    let mut oldest_value = None;
    let mut oldest_generation = 255;

    for past in pasts {
        let past_generation = past.generation(generation_resource);
        if current_generation <= past_generation && past_generation <= oldest_generation {
            oldest_value = Some(past.value());
            oldest_generation = past_generation;
        }
    }

    oldest_value
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

pub fn traverse_chain(chain_link: RustemonEvoStep) -> EvolutionStep {
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

pub fn convert_to_evolution_method(evolution: RustemonEvoMethod) -> EvolutionMethod {
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

pub fn extract_stats(stats_vec: Vec<RustemonStat>) -> Stats {
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
