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
            "SELECT * FROM {table} WHERE {fk} = ?1 AND generation >= ?2 ORDER BY generation ASC",
            table = Self::table(),
            fk = Self::fk()
        );
        db.query_row(&query, [fk_id, generation as i64], Self::on_hit)
            .optional()
    }
    fn fk() -> &'static str;
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
        Ok(Self {
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

pub struct MoveChangeRow {
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
impl TableRow for MoveChangeRow {
    fn table() -> &'static str {
        "move_changes"
    }
}
impl SelectChangeRow for MoveChangeRow {
    fn on_hit(row: &Row<'_>) -> SqlResult<Self> {
        Ok(Self {
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

    fn fk() -> &'static str {
        "move_id"
    }
}
impl InsertRow for MoveChangeRow {
    fn query() -> &'static str {
        include_str!("../sql/insert_move_change.sql")
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

pub struct TypeRow {
    pub id: i64,
    pub name: String,
    pub no_damage_to: String,
    pub half_damage_to: String,
    pub double_damage_to: String,
    pub no_damage_from: String,
    pub half_damage_from: String,
    pub double_damage_from: String,
    pub generation: u8,
}
impl TableRow for TypeRow {
    fn table() -> &'static str {
        "types"
    }
}
impl SelectRow for TypeRow {
    fn on_hit(row: &Row<'_>) -> SqlResult<Self> {
        Ok(Self {
            id: row.get(0)?,
            name: row.get(1)?,
            no_damage_to: row.get(2)?,
            half_damage_to: row.get(3)?,
            double_damage_to: row.get(4)?,
            no_damage_from: row.get(5)?,
            half_damage_from: row.get(6)?,
            double_damage_from: row.get(7)?,
            generation: row.get(8)?,
        })
    }
}
impl InsertRow for TypeRow {
    fn query() -> &'static str {
        include_str!("../sql/insert_type.sql")
    }

    fn insert(&self, statement: &mut Statement) -> SqlResult<usize> {
        statement.execute(params![
            self.id,
            self.name,
            self.no_damage_to,
            self.half_damage_to,
            self.double_damage_to,
            self.no_damage_from,
            self.half_damage_from,
            self.double_damage_from,
            self.generation
        ])
    }
}

pub struct TypeChangeRow {
    pub id: Option<i64>,
    pub no_damage_to: String,
    pub half_damage_to: String,
    pub double_damage_to: String,
    pub no_damage_from: String,
    pub half_damage_from: String,
    pub double_damage_from: String,
    pub generation: u8,
    pub type_id: i64,
}
impl TableRow for TypeChangeRow {
    fn table() -> &'static str {
        "type_changes"
    }
}
impl SelectChangeRow for TypeChangeRow {
    fn on_hit(row: &Row<'_>) -> SqlResult<Self> {
        Ok(Self {
            id: row.get(0)?,
            no_damage_to: row.get(1)?,
            half_damage_to: row.get(2)?,
            double_damage_to: row.get(3)?,
            no_damage_from: row.get(4)?,
            half_damage_from: row.get(5)?,
            double_damage_from: row.get(6)?,
            generation: row.get(7)?,
            type_id: row.get(8)?,
        })
    }

    fn fk() -> &'static str {
        "type_id"
    }
}
impl InsertRow for TypeChangeRow {
    fn query() -> &'static str {
        include_str!("../sql/insert_type_change.sql")
    }

    fn insert(&self, statement: &mut Statement) -> SqlResult<usize> {
        statement.execute(params![
            self.id,
            self.no_damage_to,
            self.half_damage_to,
            self.double_damage_to,
            self.no_damage_from,
            self.half_damage_from,
            self.double_damage_from,
            self.generation,
            self.type_id
        ])
    }
}

pub struct AbilityRow {
    pub id: i64,
    pub name: String,
    pub effect: String,
    pub generation: u8,
}
impl TableRow for AbilityRow {
    fn table() -> &'static str {
        "abilities"
    }
}
impl SelectRow for AbilityRow {
    fn on_hit(row: &Row<'_>) -> SqlResult<Self> {
        Ok(Self {
            id: row.get(0)?,
            name: row.get(1)?,
            effect: row.get(2)?,
            generation: row.get(3)?,
        })
    }
}
impl InsertRow for AbilityRow {
    fn query() -> &'static str {
        include_str!("../sql/insert_ability.sql")
    }

    fn insert(&self, statement: &mut Statement) -> SqlResult<usize> {
        statement.execute(params![self.id, self.name, self.effect, self.generation])
    }
}
