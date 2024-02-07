use std::collections::HashMap;

use anyhow::{bail, Result};
use regex::Regex;

use dirs;

use rustemon::client::{CACacheManager, RustemonClient, RustemonClientBuilder};
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
use rustemon::model::games::VersionGroup as RustemonVersion;
use rustemon::model::moves::{Move as RustemonMove, PastMoveStatValues as RustemonPastMoveStats};
use rustemon::model::pokemon::{
    Ability as RustemonAbility, AbilityEffectChange as RustemonPastAbilityEffect,
    Pokemon as RustemonPokemon, PokemonAbility as RustemonPokemonAbility,
    PokemonMove as RustemonPokemonMove, PokemonStat as RustemonStat,
    PokemonType as RustemonTypeSlot, PokemonTypePast as RustemonPastPokemonType,
    Type as RustemonType, TypeRelations as RustemonTypeRelations,
    TypeRelationsPast as RustemonPastTypeRelations,
};
use rustemon::model::resource::{Effect as RustemonEffect, VerboseEffect as RustemonVerboseEffect};

use crate::pokemon::{
    Ability, EvolutionMethod, EvolutionStep, Move, PokemonData, Stats, Type, TypeChart,
};

pub struct ApiWrapper {
    pub client: RustemonClient,
    gen_map: HashMap<String, u8>,
    gen_regex: Regex,
}

impl ApiWrapper {
    pub async fn try_new() -> Result<Self> {
        let mut cache_dir = if let Some(home_dir) = dirs::home_dir() {
            home_dir
        } else {
            bail!("Home directory not found")
        };
        cache_dir.push(".cache/dunspars/rustemon");

        let cache_manager = CACacheManager { path: cache_dir };
        let client = RustemonClientBuilder::default()
            .with_manager(cache_manager)
            .try_build()?;

        let gen_regex = Regex::new(r"generation/(?P<gen>\d+)/?$").unwrap();
        let gen_map = build_gen_map(&client, &gen_regex).await?;

        Ok(Self {
            client,
            gen_map,
            gen_regex,
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
        let generation = self.get_generation(game);

        let (primary_type, secondary_type) =
            self.get_pokemon_type(types, past_types, generation).await?;

        let learn_moves = self.get_pokemon_moves(moves, game).await?;

        let abilities = self.get_pokemon_abilities(abilities);

        Ok(PokemonData {
            name,
            primary_type,
            secondary_type,
            learn_moves,
            abilities,
            species: species.name,
            stats: extract_stats(stats),
            game: game.to_string(),
            generation,
            api: self,
        })
    }

    async fn get_pokemon_type(
        &self,
        types: Vec<RustemonTypeSlot>,
        past_types: Vec<RustemonPastPokemonType>,
        generation: u8,
    ) -> Result<(String, Option<String>)> {
        let pokemon_types = self
            .match_past(generation, past_types)
            .await
            .unwrap_or(types);

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

        Ok((primary_type, secondary_type))
    }

    async fn get_pokemon_moves(
        &self,
        moves: Vec<RustemonPokemonMove>,
        game: &str,
    ) -> Result<HashMap<String, (String, i64)>> {
        let mut learn_moves = HashMap::new();
        for move_ in moves {
            let version_group = move_
                .version_group_details
                .iter()
                .find(|vg| vg.version_group.name == game);

            if let Some(vg) = version_group {
                learn_moves.insert(
                    move_.move_.name.clone(),
                    (vg.move_learn_method.name.clone(), vg.level_learned_at),
                );
            }
        }
        Ok(learn_moves)
    }

    fn get_pokemon_abilities(&self, abilities: Vec<RustemonPokemonAbility>) -> Vec<(String, bool)> {
        abilities
            .iter()
            .map(|a| (a.ability.name.clone(), a.is_hidden))
            .collect::<Vec<_>>()
    }

    pub async fn get_type(&self, type_str: &str, generation: u8) -> Result<Type> {
        let RustemonType {
            name,
            damage_relations,
            past_damage_relations,
            ..
        } = rustemon_type::get_by_name(type_str, &self.client).await?;

        let relations = self
            .match_past(generation, past_damage_relations)
            .await
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
            generation,
            api: self,
        })
    }

    pub async fn get_move(&self, name: &str, generation: u8) -> Result<Move> {
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
            ..
        } = rustemon_moves::get_by_name(name, &self.client).await?;

        let RustemonVerboseEffect {
            mut effect,
            mut short_effect,
            ..
        } = effect_entries
            .into_iter()
            .find(|e| e.language.name == "en")
            .unwrap_or_default();

        if let Some(past_stats) = self.match_past(generation, past_values).await {
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

        if let Some(past_effects) = self.match_past(generation, effect_changes).await {
            if let Some(past_effect) = past_effects.into_iter().find(|e| e.language.name == "en") {
                effect += format!("\n\nGeneration {generation}: {}", past_effect.effect).as_str();
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
            generation,
            api: self,
        })
    }

    pub async fn get_ability(&self, name: &str, generation: u8) -> Result<Ability> {
        let RustemonAbility {
            name,
            effect_entries,
            effect_changes,
            ..
        } = rustemon_ability::get_by_name(name, &self.client).await?;

        let RustemonVerboseEffect {
            mut effect,
            short_effect,
            ..
        } = effect_entries
            .into_iter()
            .find(|e| e.language.name == "en")
            .unwrap_or_default();

        if let Some(past_effects) = self.match_past(generation, effect_changes).await {
            if let Some(past_effect) = past_effects.into_iter().find(|e| e.language.name == "en") {
                effect += format!("\n\nGeneration {generation}: {}", past_effect.effect).as_str();
            }
        }

        Ok(Ability {
            name,
            effect,
            short_effect,
            generation,
            api: self,
        })
    }

    pub fn get_generation(&self, game: &str) -> u8 {
        *self.gen_map.get(game).unwrap()
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

    async fn match_past<T: Past<U>, U>(&self, current_generation: u8, pasts: Vec<T>) -> Option<U> {
        let mut oldest_value = None;
        let mut oldest_generation = 255;

        for past in pasts {
            let past_generation = past.generation(self).await;
            if current_generation <= past_generation && past_generation <= oldest_generation {
                oldest_value = Some(past.value());
                oldest_generation = past_generation;
            }
        }

        oldest_value
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

async fn build_gen_map(client: &RustemonClient, gen_regex: &Regex) -> Result<HashMap<String, u8>> {
    let mut gen_map = HashMap::new();
    let games = get_all_games(client).await?;

    for game in games {
        let RustemonVersion { generation, .. } =
            rustemon_version::get_by_name(&game, client).await?;

        let generation = extract_gen_from_url(gen_regex, &generation.url);
        gen_map.insert(game, generation);
    }

    Ok(gen_map)
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

fn extract_gen_from_url(regex: &Regex, url: &str) -> u8 {
    let caps = regex.captures(url).unwrap();
    caps["gen"].parse::<u8>().unwrap()
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
    async fn generation(&self, api: &ApiWrapper) -> u8;
    fn value(self) -> T;
}

impl Past<Vec<RustemonTypeSlot>> for RustemonPastPokemonType {
    async fn generation(&self, api: &ApiWrapper) -> u8 {
        extract_gen_from_url(&api.gen_regex, &self.generation.url)
    }

    fn value(self) -> Vec<RustemonTypeSlot> {
        self.types
    }
}

impl Past<RustemonTypeRelations> for RustemonPastTypeRelations {
    async fn generation(&self, api: &ApiWrapper) -> u8 {
        extract_gen_from_url(&api.gen_regex, &self.generation.url)
    }

    fn value(self) -> RustemonTypeRelations {
        self.damage_relations
    }
}

impl Past<RustemonPastMoveStats> for RustemonPastMoveStats {
    async fn generation(&self, api: &ApiWrapper) -> u8 {
        api.get_generation(&self.version_group.name) - 1
    }

    fn value(self) -> RustemonPastMoveStats {
        self
    }
}

impl Past<Vec<RustemonEffect>> for RustemonPastAbilityEffect {
    async fn generation(&self, api: &ApiWrapper) -> u8 {
        api.get_generation(&self.version_group.name) - 1
    }

    fn value(self) -> Vec<RustemonEffect> {
        self.effect_entries
    }
}
