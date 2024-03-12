use crate::api::utils::capture_gen_url;
use crate::models::resource::{
    AbilityRow, GameRow, MoveChangeRow, MoveRow, MoveRowGroup, SelectRow, TypeChangeRow, TypeRow,
    TypeRowGroup,
};

use anyhow::Result;
use futures::stream::FuturesOrdered;
use futures::StreamExt;
use rusqlite::Connection;

use rustemon::client::RustemonClient;
use rustemon::games::version_group as rustemon_version;
use rustemon::moves::move_ as rustemon_move;
use rustemon::pokemon::ability as rustemon_ability;
use rustemon::pokemon::type_ as rustemon_type;

use rustemon::model::games::VersionGroup;
use rustemon::model::moves::{Move, PastMoveStatValues};
use rustemon::model::pokemon::{Ability, Type, TypeRelations, TypeRelationsPast};
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

#[allow(async_fn_in_trait)]
pub trait FetchAllEntries<T, U> {
    async fn fetch_all_entries(client: &RustemonClient, db: &Connection) -> Result<Vec<U>> {
        let identifiers = Self::fetch_all_identifiers(client).await?;
        let entry_futures: FuturesOrdered<_> = identifiers
            .iter()
            .map(|identifier| Self::fetch_entry(identifier, client))
            .collect();
        let entry_results: Vec<_> = entry_futures.collect().await;
        let mut entries = vec![];
        for entry in entry_results {
            entries.push(entry?);
        }

        Ok(Self::convert_to_rows(entries, db))
    }

    fn convert_to_rows(entries: Vec<T>, db: &Connection) -> Vec<U>;
    async fn fetch_entry(identifier: &str, client: &RustemonClient) -> Result<T>;
    async fn fetch_all_identifiers(client: &RustemonClient) -> Result<Vec<String>>;
}

pub struct GameFetcher;
impl FetchAllEntries<VersionGroup, GameRow> for GameFetcher {
    async fn fetch_all_identifiers(client: &RustemonClient) -> Result<Vec<String>> {
        Ok(rustemon_version::get_all_entries(client)
            .await?
            .into_iter()
            .map(|g| g.name)
            .collect::<Vec<String>>())
    }

    async fn fetch_entry(identifier: &str, client: &RustemonClient) -> Result<VersionGroup> {
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
impl FetchAllEntries<Move, MoveRowGroup> for MoveFetcher {
    async fn fetch_all_identifiers(client: &RustemonClient) -> Result<Vec<String>> {
        Ok(rustemon_move::get_all_entries(client)
            .await?
            .into_iter()
            .map(|g| g.name)
            .collect::<Vec<String>>())
    }

    async fn fetch_entry(identifier: &str, client: &RustemonClient) -> Result<Move> {
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
impl FetchAllEntries<Type, TypeRowGroup> for TypeFetcher {
    async fn fetch_all_identifiers(client: &RustemonClient) -> Result<Vec<String>> {
        Ok(rustemon_type::get_all_entries(client)
            .await?
            .into_iter()
            .map(|g| g.name)
            .collect::<Vec<String>>())
    }

    async fn fetch_entry(identifier: &str, client: &RustemonClient) -> Result<Type> {
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
impl FetchAllEntries<Ability, AbilityRow> for AbilityFetcher {
    async fn fetch_all_identifiers(client: &RustemonClient) -> Result<Vec<String>> {
        Ok(rustemon_ability::get_all_entries(client)
            .await?
            .into_iter()
            .map(|g| g.name)
            .collect::<Vec<String>>())
    }

    async fn fetch_entry(identifier: &str, client: &RustemonClient) -> Result<Ability> {
        Ok(rustemon_ability::get_by_name(identifier, client).await?)
    }

    fn convert_to_rows(entries: Vec<Ability>, _db: &Connection) -> Vec<AbilityRow> {
        entries
            .into_iter()
            .map(AbilityRow::from)
            .collect::<Vec<AbilityRow>>()
    }
}
