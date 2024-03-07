use crate::api::utils::capture_gen_url;
use crate::models::resource::{GameRow, MoveRow};
use rustemon::model::games::VersionGroup;
use rustemon::model::moves::Move;

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
            id: id as u16,
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
            ..
        } = value;

        MoveRow {
            id: id as u16,
            name,
            accuracy,
            power,
            pp,
        }
    }
}
