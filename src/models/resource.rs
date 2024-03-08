use anyhow::Result;
use rusqlite::{params, Connection, OptionalExtension, Result as SqlResult, Row, Statement};

pub trait FromRow<T>: Sized {
    fn from_row(value: T, current_gen: u8, db: &Connection) -> Result<Self>;
}

pub trait TableRow {
    fn table() -> &'static str;
}

pub trait InsertRow {
    fn insert_stmt(db: &Connection) -> SqlResult<Statement> {
        db.prepare(Self::query())
    }
    fn query() -> &'static str;
    fn insert(&self, statement: &mut Statement) -> SqlResult<usize>;
}

pub trait SelectRow: TableRow + Sized {
    fn select_by_name(name: &str, db: &Connection) -> SqlResult<Self> {
        let query = format!(
            "SELECT * FROM {table} WHERE name = ?1",
            table = Self::table()
        );
        db.query_row(&query, [name], Self::on_hit)
    }
    fn on_hit(row: &Row<'_>) -> SqlResult<Self>;
}

pub trait SelectChangeRow: TableRow + Sized {
    fn select_by_fk(fk_id: i64, generation: u8, db: &Connection) -> SqlResult<Option<Self>> {
        let query = format!(
            "SELECT * FROM {table} WHERE move_id = ?1 AND generation > ?2 ORDER BY generation ASC",
            table = Self::table()
        );
        db.query_row(&query, [fk_id, generation as i64], Self::on_hit)
            .optional()
    }
    fn on_hit(row: &Row<'_>) -> SqlResult<Self>;
}

pub struct GameRow {
    pub id: i64,
    pub name: String,
    pub order: u8,
    pub generation: u8,
}
impl TableRow for GameRow {
    fn table() -> &'static str {
        "games"
    }
}
impl SelectRow for GameRow {
    fn on_hit(row: &Row<'_>) -> SqlResult<Self> {
        Ok(GameRow {
            id: row.get(0)?,
            name: row.get(1)?,
            order: row.get(2)?,
            generation: row.get(3)?,
        })
    }
}
impl InsertRow for GameRow {
    fn query() -> &'static str {
        include_str!("../sql/insert_game.sql")
    }

    fn insert(&self, statement: &mut Statement) -> SqlResult<usize> {
        statement.execute(params![self.id, self.name, self.order, self.generation])
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
impl TableRow for MoveRow {
    fn table() -> &'static str {
        "moves"
    }
}
impl SelectRow for MoveRow {
    fn on_hit(row: &Row<'_>) -> SqlResult<Self> {
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
    }
}
impl InsertRow for MoveRow {
    fn query() -> &'static str {
        include_str!("../sql/insert_move.sql")
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
impl TableRow for ChangeMoveValueRow {
    fn table() -> &'static str {
        "change_move_value"
    }
}
impl SelectChangeRow for ChangeMoveValueRow {
    fn on_hit(row: &Row<'_>) -> SqlResult<Self> {
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
    }
}
impl InsertRow for ChangeMoveValueRow {
    fn query() -> &'static str {
        include_str!("../sql/insert_change_move_value.sql")
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
