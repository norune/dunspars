use std::collections::HashMap;

use anyhow::Result;
use regex::Regex;

use rustemon::client::RustemonClient;
use rustemon::games::version_group as rustemon_version;
use rustemon::moves as rustemon_moves;
use rustemon::pokemon::pokemon_species as rustemon_species;
use rustemon::pokemon::{
    ability as rustemon_ability, pokemon as rustemon_pokemon, type_ as rustemon_type,
};
use rustemon::Follow;

use rustemon::model::evolution::{
    ChainLink as RustemonChainLink, EvolutionDetail as RustemonEvolution,
};
use rustemon::model::games::VersionGroup as RustemonVersion;
use rustemon::model::moves::Move as RustemonMove;
use rustemon::model::pokemon::Ability as RustemonAbility;
use rustemon::model::pokemon::Pokemon as RustemonPokemon;
use rustemon::model::pokemon::PokemonStat as RustemonStat;

use crate::pokemon::{
    Ability, EvolutionMethod, EvolutionStep, Move, PokemonData, Stats, Type, TypeChart,
};

#[derive(Default)]
pub struct ApiWrapper {
    client: RustemonClient,
}

impl ApiWrapper {
    pub async fn get_pokemon(&self, pokemon: &str, version: &str) -> Result<PokemonData> {
        let RustemonPokemon {
            name,
            types,
            moves,
            stats,
            abilities,
            species,
            ..
        } = rustemon_pokemon::get_by_name(pokemon, &self.client).await?;
        let generation = self.get_generation(version).await?;

        let mut types = types.iter();
        let primary_type = types.find(|t| t.slot == 1).unwrap().type_.name.clone();
        let secondary_type = types.find(|t| t.slot == 2).map(|t| t.type_.name.clone());

        let mut learn_moves = HashMap::new();
        moves.iter().for_each(|mv| {
            let version_group = mv
                .version_group_details
                .iter()
                .find(|vg| vg.version_group.name == version);
            if let Some(vg) = version_group {
                learn_moves.insert(
                    mv.move_.name.clone(),
                    (vg.move_learn_method.name.clone(), vg.level_learned_at),
                );
            }
        });
        let abilities = abilities
            .iter()
            .map(|a| (a.ability.name.clone(), a.is_hidden))
            .collect::<Vec<_>>();

        Ok(PokemonData {
            name,
            primary_type,
            secondary_type,
            learn_moves,
            abilities,
            species: species.name,
            stats: extract_stats(stats),
            game: version.to_string(),
            generation,
            api: self,
        })
    }

    pub async fn get_type(&self, type_str: &str) -> Result<Type> {
        let type_ = rustemon_type::get_by_name(type_str, &self.client).await?;
        let mut offense_chart = HashMap::new();
        let mut defense_chart = HashMap::new();

        type_.damage_relations.no_damage_to.iter().for_each(|t| {
            offense_chart.insert(t.name.to_string(), 0.0);
        });
        type_.damage_relations.half_damage_to.iter().for_each(|t| {
            offense_chart.insert(t.name.to_string(), 0.5);
        });
        type_
            .damage_relations
            .double_damage_to
            .iter()
            .for_each(|t| {
                offense_chart.insert(t.name.to_string(), 2.0);
            });

        type_.damage_relations.no_damage_from.iter().for_each(|t| {
            defense_chart.insert(t.name.to_string(), 0.0);
        });
        type_
            .damage_relations
            .half_damage_from
            .iter()
            .for_each(|t| {
                defense_chart.insert(t.name.to_string(), 0.5);
            });
        type_
            .damage_relations
            .double_damage_from
            .iter()
            .for_each(|t| {
                defense_chart.insert(t.name.to_string(), 2.0);
            });

        Ok(Type {
            name: type_.name,
            offense_chart: TypeChart::new(offense_chart),
            defense_chart: TypeChart::new(defense_chart),
            api: self,
        })
    }

    pub async fn get_move(&self, name: &str) -> Result<Move> {
        let RustemonMove {
            name,
            accuracy,
            power,
            pp,
            damage_class,
            type_,
            effect_entries,
            ..
        } = rustemon_moves::move_::get_by_name(name, &self.client).await?;
        let effect_entry = effect_entries
            .into_iter()
            .find(|e| e.language.name == "en")
            .unwrap_or_default();

        Ok(Move {
            name,
            accuracy,
            power,
            pp,
            damage_class: damage_class.name,
            type_: type_.name,
            effect: effect_entry.effect,
            effect_short: effect_entry.short_effect,
            api: self,
        })
    }

    pub async fn get_ability(&self, name: &str) -> Result<Ability> {
        let RustemonAbility {
            name,
            effect_entries,
            ..
        } = rustemon_ability::get_by_name(name, &self.client).await?;
        let effect_entry = effect_entries
            .into_iter()
            .find(|e| e.language.name == "en")
            .unwrap_or_default();

        Ok(Ability {
            name,
            effect: effect_entry.effect,
            effect_short: effect_entry.short_effect,
            api: self,
        })
    }

    pub async fn get_generation(&self, version: &str) -> Result<u8> {
        let RustemonVersion { generation, .. } =
            rustemon_version::get_by_name(version, &self.client).await?;

        extract_gen_from_url(&generation.url)
    }

    pub async fn get_evolution_steps(&self, species: &str) -> Result<EvolutionStep> {
        let evolution_chain = rustemon_species::get_by_name(species, &self.client)
            .await?
            .evolution_chain
            .unwrap()
            .follow(&self.client)
            .await?;
        let evolution_step = traverse_chain(evolution_chain.chain);

        Ok(evolution_step)
    }
}

fn traverse_chain(chain_link: RustemonChainLink) -> EvolutionStep {
    let evolution_method = chain_link
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

        EvolutionStep::new(chain_link.species.name, evolution_method, evolves_to)
    } else {
        EvolutionStep::new(chain_link.species.name, evolution_method, vec![])
    }
}

fn convert_to_evolution_method(evolution: RustemonEvolution) -> EvolutionMethod {
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

fn extract_gen_from_url(url: &str) -> Result<u8> {
    let gen_url_regex = Regex::new(r"generation/(?P<gen>\d+)/?$").unwrap();
    let caps = gen_url_regex.captures(url).unwrap();
    let generation = caps["gen"].parse::<u8>()?;
    Ok(generation)
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
