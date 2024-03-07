use super::Game;

pub struct GameRow {
    pub id: u16,
    pub name: String,
    pub order: u8,
    pub generation: u8,
}

impl From<GameRow> for Game {
    fn from(row: GameRow) -> Self {
        let GameRow {
            name,
            order,
            generation,
            ..
        } = row;
        Game::new(name, order, generation)
    }
}

pub struct MoveRow {
    pub id: u16,
    pub name: String,
    pub accuracy: Option<i64>,
    pub power: Option<i64>,
    pub pp: Option<i64>,
}
