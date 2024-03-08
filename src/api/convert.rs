use crate::api::utils::capture_gen_url;
use crate::models::resource::{GameRow, MoveChangeRow, MoveRow, SelectRow, TypeChangeRow, TypeRow};

use rusqlite::Connection;
use rustemon::model::games::VersionGroup;
use rustemon::model::moves::{Move, PastMoveStatValues};
use rustemon::model::pokemon::{Type, TypeRelations, TypeRelationsPast};
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

        GameRow {
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

        MoveRow {
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
        let generation = Self::game_to_gen(&version_group.name, db);

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
