use crate::api::utils::capture_gen_url;
use crate::models::resource::{ChangeMoveValueRow, GameRow, MoveRow, SelectRow};
use rusqlite::Connection;
use rustemon::model::games::VersionGroup;
use rustemon::model::moves::{Move, PastMoveStatValues};
use rustemon::model::resource::VerboseEffect;

pub trait FromChange<T> {
    fn game_to_gen(game: &str, db: &Connection) -> u8 {
        let game = GameRow::select_by_name(game, db).unwrap();
        game.generation
    }
    fn from_change(value: T, id: i64, db: &Connection) -> Self;
}

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

impl FromChange<&PastMoveStatValues> for ChangeMoveValueRow {
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
