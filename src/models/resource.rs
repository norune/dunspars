use super::{Game, Move};
use anyhow::Result;
use rusqlite::{params, Connection, OptionalExtension, Result as SqlResult, Statement};

pub trait Row {
    fn insert_stmt(db: &Connection) -> SqlResult<Statement>;
    fn insert(&self, statement: &mut Statement) -> SqlResult<usize>;
}

pub trait FromGen<T>: Sized {
    fn from_gen(value: T, current_gen: u8, db: &Connection) -> Result<Self>;
}

pub struct GameRow {
    pub id: i64,
    pub name: String,
    pub order: u8,
    pub generation: u8,
}
impl Row for GameRow {
    fn insert_stmt(db: &Connection) -> SqlResult<Statement> {
        db.prepare(include_str!("../sql/insert_game.sql"))
    }
    fn insert(&self, statement: &mut Statement) -> SqlResult<usize> {
        statement.execute(params![self.id, self.name, self.order, self.generation])
    }
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
    pub id: i64,
    pub name: String,
    pub power: Option<i64>,
    pub accuracy: Option<i64>,
    pub pp: Option<i64>,
    pub effect_chance: Option<i64>,
    pub effect: String,
    pub type_: String,
    pub damage_class: String,
    pub generation: u8,
}
impl MoveRow {
    pub fn from_name(name: &str, db: &Connection) -> SqlResult<Self> {
        let query = "SELECT * FROM moves WHERE name = ?1";
        db.query_row(query, [name], |row| {
            Ok(MoveRow {
                id: row.get(0)?,
                name: row.get(1)?,
                power: row.get(2)?,
                accuracy: row.get(3)?,
                pp: row.get(4)?,
                effect_chance: row.get(5)?,
                effect: row.get(6)?,
                type_: row.get(7)?,
                damage_class: row.get(8)?,
                generation: row.get(9)?,
            })
        })
    }
}
impl Row for MoveRow {
    fn insert_stmt(db: &Connection) -> SqlResult<Statement> {
        db.prepare(include_str!("../sql/insert_move.sql"))
    }

    fn insert(&self, statement: &mut Statement) -> SqlResult<usize> {
        statement.execute(params![
            self.id,
            self.name,
            self.power,
            self.accuracy,
            self.pp,
            self.damage_class,
            self.type_,
            self.effect,
            self.effect_chance,
            self.generation
        ])
    }
}
impl FromGen<MoveRow> for Move {
    fn from_gen(value: MoveRow, current_gen: u8, db: &Connection) -> Result<Self> {
        let MoveRow {
            id,
            name,
            mut power,
            mut accuracy,
            mut pp,
            mut effect_chance,
            effect,
            mut type_,
            damage_class,
            generation,
        } = value;

        let change_row = ChangeMoveValueRow::from_fk(id, current_gen, db)?;
        if let Some(change) = change_row {
            power = change.power.or(power);
            accuracy = change.accuracy.or(accuracy);
            pp = change.pp.or(pp);
            effect_chance = change.effect_chance.or(effect_chance);

            if let Some(t) = change.type_ {
                type_ = t;
            }
        }

        Ok(Move {
            name,
            accuracy,
            power,
            pp,
            damage_class,
            type_,
            effect,
            effect_chance,
            generation,
        })
    }
}

pub struct ChangeMoveValueRow {
    pub id: Option<i64>,
    pub power: Option<i64>,
    pub accuracy: Option<i64>,
    pub pp: Option<i64>,
    pub effect_chance: Option<i64>,
    pub effect: Option<String>,
    pub type_: Option<String>,
    pub generation: u8,
    pub move_id: i64,
}
impl ChangeMoveValueRow {
    pub fn from_fk(move_id: i64, generation: u8, db: &Connection) -> SqlResult<Option<Self>> {
        let query = "SELECT * FROM change_move_value WHERE move_id = ?1 AND generation > ?2 ORDER BY generation ASC";
        db.query_row(query, [move_id, generation as i64], |row| {
            Ok(ChangeMoveValueRow {
                id: row.get(0)?,
                power: row.get(1)?,
                accuracy: row.get(2)?,
                pp: row.get(3)?,
                effect_chance: row.get(4)?,
                effect: row.get(5)?,
                type_: row.get(6)?,
                generation: row.get(7)?,
                move_id: row.get(8)?,
            })
        })
        .optional()
    }
}
impl Row for ChangeMoveValueRow {
    fn insert_stmt(db: &Connection) -> SqlResult<Statement> {
        db.prepare(include_str!("../sql/insert_change_move_value.sql"))
    }
    fn insert(&self, statement: &mut Statement) -> SqlResult<usize> {
        statement.execute(params![
            self.id,
            self.power,
            self.accuracy,
            self.pp,
            self.effect_chance,
            self.effect,
            self.type_,
            self.generation,
            self.move_id
        ])
    }
}
