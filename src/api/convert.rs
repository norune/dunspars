use super::game_to_gen;
use crate::models::database::{
    AbilityRow, GameRow, MoveChangeRow, MoveRow, PokemonAbilityRow, PokemonMoveRow, PokemonRow,
    PokemonTypeChangeRow, SpeciesRow, TypeChangeRow, TypeRow,
};
use crate::models::{EvolutionMethod, EvolutionStep};

use std::sync::OnceLock;

use anyhow::{anyhow, Result};
use regex::Regex;
use rusqlite::Connection;

use rustemon::model::evolution::{ChainLink, EvolutionDetail};
use rustemon::model::games::VersionGroup;
use rustemon::model::moves::{Move, PastMoveStatValues};
use rustemon::model::pokemon::{
    Ability, Pokemon, PokemonAbility, PokemonMove, PokemonSpecies, PokemonStat, PokemonType,
    PokemonTypePast, Type, TypeRelations, TypeRelationsPast,
};
use rustemon::model::resource::{NamedApiResource, VerboseEffect};

trait GetEffectEntry {
    fn get_effect(&self) -> Option<String>;
}

impl GetEffectEntry for Vec<VerboseEffect> {
    fn get_effect(&self) -> Option<String> {
        self.iter()
            .find(|e| e.language.name == "en")
            .map(|ve| ve.effect.clone())
    }
}

impl From<VersionGroup> for GameRow {
    fn from(value: VersionGroup) -> Self {
        let VersionGroup {
            id,
            name,
            order,
            generation,
            ..
        } = value;
        let generation = capture_url_gen(&generation.url).unwrap();

        Self {
            id,
            name,
            order: order as u8,
            generation,
        }
    }
}

impl From<Move> for MoveRow {
    fn from(value: Move) -> Self {
        let Move {
            id,
            name,
            accuracy,
            power,
            pp,
            damage_class,
            type_,
            effect_chance,
            effect_entries,
            generation,
            ..
        } = value;

        let effect = effect_entries.get_effect().unwrap_or_default();

        Self {
            id,
            name,
            accuracy,
            power,
            pp,
            damage_class: damage_class.name,
            type_: type_.name,
            effect,
            effect_chance,
            generation: capture_url_gen(&generation.url).unwrap(),
        }
    }
}
pub trait FromChange<T> {
    fn from_change(value: T, id: i64, db: &Connection) -> Self;
}

impl FromChange<&PastMoveStatValues> for MoveChangeRow {
    fn from_change(value: &PastMoveStatValues, id: i64, db: &Connection) -> Self {
        let PastMoveStatValues {
            accuracy,
            effect_chance,
            power,
            pp,
            effect_entries,
            type_,
            version_group,
        } = value;

        let effect = effect_entries.get_effect();
        let type_ = type_.clone().map(|t| t.name);

        // For whatever reason, pokeapi denotes past move values
        // on the generation when they stop being applicable.
        // e.g. Tackle 35 power 95 accuracy is applicable to gen 1-4
        // However, pokeapi labels this past value as gen 5.
        let generation = game_to_gen(&version_group.name, db) - 1;

        Self {
            id: None,
            accuracy: *accuracy,
            power: *power,
            pp: *pp,
            effect_chance: *effect_chance,
            type_,
            effect,
            generation,
            move_id: id,
        }
    }
}

trait GetTypes {
    fn get_types(&self) -> String;
}

impl GetTypes for Vec<NamedApiResource<Type>> {
    fn get_types(&self) -> String {
        self.iter()
            .map(|r| r.name.clone())
            .collect::<Vec<String>>()
            .join(",")
    }
}

impl From<Type> for TypeRow {
    fn from(value: Type) -> Self {
        let Type {
            id,
            name,
            damage_relations,
            generation,
            ..
        } = value;

        let TypeRelations {
            no_damage_to,
            half_damage_to,
            double_damage_to,
            no_damage_from,
            half_damage_from,
            double_damage_from,
        } = damage_relations;
        let generation = capture_url_gen(&generation.url).unwrap();

        Self {
            id,
            name,
            no_damage_to: no_damage_to.get_types(),
            half_damage_to: half_damage_to.get_types(),
            double_damage_to: double_damage_to.get_types(),
            no_damage_from: no_damage_from.get_types(),
            half_damage_from: half_damage_from.get_types(),
            double_damage_from: double_damage_from.get_types(),
            generation,
        }
    }
}

impl FromChange<&TypeRelationsPast> for TypeChangeRow {
    fn from_change(value: &TypeRelationsPast, id: i64, _db: &Connection) -> Self {
        let TypeRelationsPast {
            generation,
            damage_relations,
        } = value;

        let TypeRelations {
            no_damage_to,
            half_damage_to,
            double_damage_to,
            no_damage_from,
            half_damage_from,
            double_damage_from,
        } = damage_relations;
        let generation = capture_url_gen(&generation.url).unwrap();

        Self {
            id: None,
            no_damage_to: no_damage_to.get_types(),
            half_damage_to: half_damage_to.get_types(),
            double_damage_to: double_damage_to.get_types(),
            no_damage_from: no_damage_from.get_types(),
            half_damage_from: half_damage_from.get_types(),
            double_damage_from: double_damage_from.get_types(),
            generation,
            type_id: id,
        }
    }
}

impl From<Ability> for AbilityRow {
    fn from(value: Ability) -> Self {
        let Ability {
            id,
            name,
            generation,
            effect_entries,
            ..
        } = value;
        let generation = capture_url_gen(&generation.url).unwrap();
        let effect = effect_entries.get_effect().unwrap_or_default();

        Self {
            id,
            name,
            effect,
            generation,
        }
    }
}

impl From<PokemonSpecies> for SpeciesRow {
    fn from(value: PokemonSpecies) -> Self {
        let PokemonSpecies {
            id,
            name,
            is_baby,
            is_legendary,
            is_mythical,
            evolution_chain,
            ..
        } = value;
        let evolution_id = evolution_chain.map(|c| capture_url_id(&c.url).unwrap() as i64);

        Self {
            id,
            name,
            is_baby,
            is_legendary,
            is_mythical,
            evolution_id,
        }
    }
}

impl From<ChainLink> for EvolutionStep {
    fn from(chain_link: ChainLink) -> Self {
        let evolution_methods = chain_link
            .evolution_details
            .into_iter()
            .map(EvolutionMethod::from)
            .collect();

        if !chain_link.evolves_to.is_empty() {
            let evolves_to = chain_link
                .evolves_to
                .into_iter()
                .map(EvolutionStep::from)
                .collect();

            EvolutionStep::new(chain_link.species.name, evolution_methods, evolves_to)
        } else {
            EvolutionStep::new(chain_link.species.name, evolution_methods, vec![])
        }
    }
}

impl From<EvolutionDetail> for EvolutionMethod {
    fn from(evolution: EvolutionDetail) -> Self {
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
}

impl FromChange<&PokemonAbility> for PokemonAbilityRow {
    fn from_change(value: &PokemonAbility, id: i64, _db: &Connection) -> Self {
        let PokemonAbility {
            is_hidden,
            slot,
            ability,
        } = value;

        Self {
            id: None,
            ability_id: capture_url_id(&ability.url).unwrap(),
            is_hidden: *is_hidden,
            slot: *slot,
            pokemon_id: id,
        }
    }
}

impl FromChange<&PokemonMove> for Vec<PokemonMoveRow> {
    fn from_change(value: &PokemonMove, id: i64, db: &Connection) -> Self {
        let PokemonMove {
            move_,
            version_group_details,
        } = value;

        let mut move_rows = vec![];
        for vg in version_group_details {
            move_rows.push(PokemonMoveRow {
                id: None,
                move_id: capture_url_id(&move_.url).unwrap(),
                learn_method: vg.move_learn_method.name.clone(),
                learn_level: vg.level_learned_at,
                generation: game_to_gen(&vg.version_group.name, db),
                pokemon_id: id,
            })
        }

        move_rows
    }
}

impl FromChange<&PokemonTypePast> for PokemonTypeChangeRow {
    fn from_change(value: &PokemonTypePast, id: i64, _db: &Connection) -> Self {
        let PokemonTypePast { generation, types } = value;
        let generation = capture_url_gen(&generation.url).unwrap();

        let primary_type = get_type(types, 1).unwrap();
        let secondary_type = get_type(types, 2);

        Self {
            id: None,
            primary_type,
            secondary_type,
            generation,
            pokemon_id: id,
        }
    }
}

impl From<Pokemon> for PokemonRow {
    fn from(value: Pokemon) -> Self {
        let Pokemon {
            id,
            name,
            species,
            stats,
            types,
            ..
        } = value;

        let primary_type = get_type(&types, 1).unwrap();
        let secondary_type = get_type(&types, 2);
        let species_id = capture_url_id(&species.url).unwrap();

        let hp = get_stat(&stats, "hp");
        let attack = get_stat(&stats, "attack");
        let defense = get_stat(&stats, "defense");
        let special_attack = get_stat(&stats, "special-attack");
        let special_defense = get_stat(&stats, "special-defense");
        let speed = get_stat(&stats, "speed");

        Self {
            id,
            name,
            primary_type,
            secondary_type,
            hp,
            attack,
            defense,
            special_attack,
            special_defense,
            speed,
            species_id,
        }
    }
}

fn get_type(types: &[PokemonType], slot: i64) -> Option<String> {
    types
        .iter()
        .find(|t| t.slot == slot)
        .map(|t| t.type_.name.clone())
}

fn get_stat(stats: &[PokemonStat], stat: &str) -> i64 {
    stats
        .iter()
        .find(|s| s.stat.name == stat)
        .map(|s| s.base_stat)
        .unwrap_or_default()
}

// Regex compilation is expensive, so we're compiling it just once here.
fn url_id_regex() -> &'static Regex {
    static ID_URL_REGEX: OnceLock<Regex> = OnceLock::new();
    ID_URL_REGEX.get_or_init(|| Regex::new(r"/(?P<id>\d+)/?$").unwrap())
}
fn url_gen_regex() -> &'static Regex {
    static GEN_URL_REGEX: OnceLock<Regex> = OnceLock::new();
    GEN_URL_REGEX.get_or_init(|| {
        // PokéAPI keeps generation names in Roman numerals.
        // Might be quicker to just take it from resource urls via regex instead.
        Regex::new(r"generation/(?P<gen>\d+)/?$").unwrap()
    })
}

pub fn capture_url_id(url: &str) -> Result<i64> {
    if let Some(caps) = url_id_regex().captures(url) {
        Ok(caps["id"].parse::<i64>()?)
    } else {
        Err(anyhow!("ID not found in resource url"))
    }
}

fn capture_url_gen(url: &str) -> Result<u8> {
    if let Some(caps) = url_gen_regex().captures(url) {
        Ok(caps["gen"].parse::<u8>()?)
    } else {
        Err(anyhow!("Generation not found in resource url"))
    }
}
