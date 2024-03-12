use crate::api::utils::capture_gen_url;
use crate::models::resource::{
    AbilityRow, EvolutionRow, GameRow, MoveChangeRow, MoveRow, MoveRowGroup, PokemonAbilityRow,
    PokemonMoveRow, PokemonRow, PokemonRowGroup, PokemonTypeChangeRow, SelectRow, SpeciesRow,
    TypeChangeRow, TypeRow, TypeRowGroup,
};
use crate::models::{EvolutionMethod, EvolutionStep};

use std::sync::OnceLock;

use anyhow::{anyhow, Result};
use futures::stream::FuturesOrdered;
use futures::StreamExt;
use regex::Regex;
use rusqlite::Connection;

use rustemon::client::RustemonClient;
use rustemon::evolution::evolution_chain as rustemon_evolution;
use rustemon::games::version_group as rustemon_version;
use rustemon::moves::move_ as rustemon_move;
use rustemon::pokemon::ability as rustemon_ability;
use rustemon::pokemon::pokemon as rustemon_pokemon;
use rustemon::pokemon::pokemon_species as rustemon_species;
use rustemon::pokemon::type_ as rustemon_type;

use rustemon::model::evolution::{ChainLink, EvolutionChain, EvolutionDetail};
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
        let generation = capture_gen_url(&generation.url).unwrap();

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
            generation: capture_gen_url(&generation.url).unwrap(),
        }
    }
}

pub trait FromChange<T> {
    fn game_to_gen(game: &str, db: &Connection) -> u8 {
        let game = GameRow::select_by_name(game, db).unwrap();
        game.generation
    }
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
        let generation = Self::game_to_gen(&version_group.name, db) - 1;

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
        let generation = capture_gen_url(&generation.url).unwrap();

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
        let generation = capture_gen_url(&generation.url).unwrap();

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
        let generation = capture_gen_url(&generation.url).unwrap();
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
            name: ability.name.clone(),
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
                name: move_.name.clone(),
                learn_method: vg.move_learn_method.name.clone(),
                learn_level: vg.level_learned_at,
                generation: Self::game_to_gen(&vg.version_group.name, db),
                pokemon_id: id,
            })
        }

        move_rows
    }
}

impl FromChange<&PokemonTypePast> for PokemonTypeChangeRow {
    fn from_change(value: &PokemonTypePast, id: i64, _db: &Connection) -> Self {
        let PokemonTypePast { generation, types } = value;
        let generation = capture_gen_url(&generation.url).unwrap();

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

        let attack = get_stat(&stats, "attack");
        let defense = get_stat(&stats, "defense");
        let special_attack = get_stat(&stats, "special_attack");
        let special_defense = get_stat(&stats, "special_defense");
        let speed = get_stat(&stats, "speed");

        Self {
            id,
            name,
            primary_type,
            secondary_type,
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

fn url_id_regex() -> &'static Regex {
    static ID_URL_REGEX: OnceLock<Regex> = OnceLock::new();
    ID_URL_REGEX.get_or_init(|| Regex::new(r"/(?P<id>\d+)/?$").unwrap())
}

fn capture_url_id(url: &str) -> Result<i64> {
    if let Some(caps) = url_id_regex().captures(url) {
        Ok(caps["id"].parse::<i64>()?)
    } else {
        Err(anyhow!("ID not found in resource url"))
    }
}

#[allow(async_fn_in_trait)]
pub trait FetchIdentifiers {
    async fn fetch_all_identifiers(client: &RustemonClient) -> Result<Vec<String>>;
}

#[allow(async_fn_in_trait)]
pub trait FetchEntries<I, T, U> {
    async fn fetch_all_entries(
        identifiers: Vec<I>,
        client: &RustemonClient,
        db: &Connection,
    ) -> Result<Vec<U>> {
        // Entry retrieval needs to be done in chunks because sending too many TCP requests
        // concurrently can cause "tcp open error: Too many open files (os error 24)"
        let chunked_identifiers = identifiers.chunks(100);
        let mut entries = vec![];

        for chunk in chunked_identifiers {
            let entry_futures: FuturesOrdered<_> = chunk
                .iter()
                .map(|identifier| Self::fetch_entry(identifier, client))
                .collect();
            let entry_results: Vec<_> = entry_futures.collect().await;
            for entry in entry_results {
                entries.push(entry?);
            }
        }

        Ok(Self::convert_to_rows(entries, db))
    }

    fn convert_to_rows(entries: Vec<T>, db: &Connection) -> Vec<U>;
    async fn fetch_entry(identifier: &I, client: &RustemonClient) -> Result<T>;
}

pub struct GameFetcher;
impl FetchIdentifiers for GameFetcher {
    async fn fetch_all_identifiers(client: &RustemonClient) -> Result<Vec<String>> {
        Ok(rustemon_version::get_all_entries(client)
            .await?
            .into_iter()
            .map(|g| g.name)
            .collect::<Vec<String>>())
    }
}
impl FetchEntries<String, VersionGroup, GameRow> for GameFetcher {
    async fn fetch_entry(identifier: &String, client: &RustemonClient) -> Result<VersionGroup> {
        Ok(rustemon_version::get_by_name(identifier, client).await?)
    }

    fn convert_to_rows(entries: Vec<VersionGroup>, _db: &Connection) -> Vec<GameRow> {
        entries
            .into_iter()
            .map(GameRow::from)
            .collect::<Vec<GameRow>>()
    }
}

pub struct MoveFetcher;
impl FetchIdentifiers for MoveFetcher {
    async fn fetch_all_identifiers(client: &RustemonClient) -> Result<Vec<String>> {
        Ok(rustemon_move::get_all_entries(client)
            .await?
            .into_iter()
            .map(|g| g.name)
            .collect::<Vec<String>>())
    }
}
impl FetchEntries<String, Move, MoveRowGroup> for MoveFetcher {
    async fn fetch_entry(identifier: &String, client: &RustemonClient) -> Result<Move> {
        Ok(rustemon_move::get_by_name(identifier, client).await?)
    }

    fn convert_to_rows(entries: Vec<Move>, db: &Connection) -> Vec<MoveRowGroup> {
        let mut move_data = vec![];

        for move_ in entries {
            for past_value in move_.past_values.iter() {
                let change_move = MoveChangeRow::from_change(past_value, move_.id, db);
                move_data.push(MoveRowGroup::MoveChangeRow(change_move));
            }

            let move_ = MoveRow::from(move_);
            move_data.push(MoveRowGroup::MoveRow(move_));
        }

        move_data
    }
}

pub struct TypeFetcher;
impl FetchIdentifiers for TypeFetcher {
    async fn fetch_all_identifiers(client: &RustemonClient) -> Result<Vec<String>> {
        Ok(rustemon_type::get_all_entries(client)
            .await?
            .into_iter()
            .map(|g| g.name)
            .collect::<Vec<String>>())
    }
}
impl FetchEntries<String, Type, TypeRowGroup> for TypeFetcher {
    async fn fetch_entry(identifier: &String, client: &RustemonClient) -> Result<Type> {
        Ok(rustemon_type::get_by_name(identifier, client).await?)
    }

    fn convert_to_rows(entries: Vec<Type>, db: &Connection) -> Vec<TypeRowGroup> {
        let mut type_data = vec![];
        for type_ in entries {
            for past_type in type_.past_damage_relations.iter() {
                let change_move = TypeChangeRow::from_change(past_type, type_.id, db);
                type_data.push(TypeRowGroup::TypeChangeRow(change_move));
            }

            let move_ = TypeRow::from(type_);
            type_data.push(TypeRowGroup::TypeRow(move_));
        }
        type_data
    }
}

pub struct AbilityFetcher;
impl FetchIdentifiers for AbilityFetcher {
    async fn fetch_all_identifiers(client: &RustemonClient) -> Result<Vec<String>> {
        Ok(rustemon_ability::get_all_entries(client)
            .await?
            .into_iter()
            .map(|g| g.name)
            .collect::<Vec<String>>())
    }
}
impl FetchEntries<String, Ability, AbilityRow> for AbilityFetcher {
    async fn fetch_entry(identifier: &String, client: &RustemonClient) -> Result<Ability> {
        Ok(rustemon_ability::get_by_name(identifier, client).await?)
    }

    fn convert_to_rows(entries: Vec<Ability>, _db: &Connection) -> Vec<AbilityRow> {
        entries
            .into_iter()
            .map(AbilityRow::from)
            .collect::<Vec<AbilityRow>>()
    }
}

pub struct SpeciesFetcher;
impl FetchIdentifiers for SpeciesFetcher {
    async fn fetch_all_identifiers(client: &RustemonClient) -> Result<Vec<String>> {
        Ok(rustemon_species::get_all_entries(client)
            .await?
            .into_iter()
            .map(|g| g.name)
            .collect::<Vec<String>>())
    }
}
impl FetchEntries<String, PokemonSpecies, SpeciesRow> for SpeciesFetcher {
    async fn fetch_entry(identifier: &String, client: &RustemonClient) -> Result<PokemonSpecies> {
        Ok(rustemon_species::get_by_name(identifier, client).await?)
    }
    fn convert_to_rows(entries: Vec<PokemonSpecies>, _db: &Connection) -> Vec<SpeciesRow> {
        entries
            .into_iter()
            .map(SpeciesRow::from)
            .collect::<Vec<SpeciesRow>>()
    }
}

pub struct EvolutionFetcher;
impl FetchEntries<i64, EvolutionChain, EvolutionRow> for EvolutionFetcher {
    async fn fetch_entry(identifier: &i64, client: &RustemonClient) -> Result<EvolutionChain> {
        Ok(rustemon_evolution::get_by_id(*identifier, client).await?)
    }

    fn convert_to_rows(entries: Vec<EvolutionChain>, _db: &Connection) -> Vec<EvolutionRow> {
        let mut evo_data = vec![];
        for evolution in entries {
            let evolution_step = EvolutionStep::from(evolution.chain);
            let serialized_step = serde_json::to_string(&evolution_step).unwrap();
            let evolution_row = EvolutionRow {
                id: evolution.id,
                evolution: serialized_step,
            };
            evo_data.push(evolution_row);
        }
        evo_data
    }
}

pub struct PokemonFetcher;
impl FetchIdentifiers for PokemonFetcher {
    async fn fetch_all_identifiers(client: &RustemonClient) -> Result<Vec<String>> {
        Ok(rustemon_pokemon::get_all_entries(client)
            .await?
            .into_iter()
            .map(|g| g.name)
            .collect::<Vec<String>>())
    }
}
impl FetchEntries<String, Pokemon, PokemonRowGroup> for PokemonFetcher {
    async fn fetch_entry(identifier: &String, client: &RustemonClient) -> Result<Pokemon> {
        Ok(rustemon_pokemon::get_by_name(identifier, client).await?)
    }

    fn convert_to_rows(entries: Vec<Pokemon>, db: &Connection) -> Vec<PokemonRowGroup> {
        let mut pokemon_data = vec![];
        for pokemon in entries {
            for ability in pokemon.abilities.iter() {
                let ability_row = PokemonAbilityRow::from_change(ability, pokemon.id, db);
                pokemon_data.push(PokemonRowGroup::PokemonAbilityRow(ability_row));
            }

            for move_ in pokemon.moves.iter() {
                let move_rows = Vec::<PokemonMoveRow>::from_change(move_, pokemon.id, db);
                pokemon_data.append(
                    &mut move_rows
                        .into_iter()
                        .map(PokemonRowGroup::PokemonMoveRow)
                        .collect(),
                );
            }

            for past_type in pokemon.past_types.iter() {
                let change_row = PokemonTypeChangeRow::from_change(past_type, pokemon.id, db);
                pokemon_data.push(PokemonRowGroup::PokemonTypeChangeRow(change_row));
            }

            let pokemon_row = PokemonRow::from(pokemon);
            pokemon_data.push(PokemonRowGroup::PokemonRow(pokemon_row));
        }
        pokemon_data
    }
}
