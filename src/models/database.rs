use anyhow::{bail, Result};
use rusqlite::{params, Connection, OptionalExtension, Result as SqlResult, Row};

pub trait FromRow<T>: Sized {
    fn from_row(value: T, current_gen: u8, db: &Connection) -> Result<Self>;
}

pub trait TableRow {
    fn table() -> &'static str;
    fn label() -> &'static str;
}

pub trait InsertRow {
    fn insert(&self, db: &Connection) -> SqlResult<usize>;
}

pub trait SelectRow: TableRow + Sized {
    fn select_by_name(name: &str, db: &Connection) -> SqlResult<Self> {
        let query = format!(
            "SELECT * FROM {table} WHERE name = ?1",
            table = Self::table()
        );
        db.query_row(&query, [name], Self::on_hit)
    }
    fn select_by_id(id: i64, db: &Connection) -> SqlResult<Self> {
        let query = format!("SELECT * FROM {table} WHERE id = ?1", table = Self::table());
        db.query_row(&query, [id], Self::on_hit)
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

pub trait SelectAllNames: TableRow {
    fn select_all_names(db: &Connection) -> SqlResult<Vec<String>> {
        let mut statement = db.prepare_cached(&format!(
            "SELECT name FROM {table} ORDER BY id",
            table = Self::table()
        ))?;
        let rows = statement.query_map([], |row| row.get(0))?;

        let mut names = vec![];
        for row in rows {
            names.push(row?);
        }

        Ok(names)
    }
}

pub enum ResourceResult {
    Valid,
    Invalid(Vec<String>),
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
    fn label() -> &'static str {
        "Game"
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
    fn insert(&self, db: &Connection) -> SqlResult<usize> {
        let mut statement = db.prepare_cached(include_str!("../sql/insert_game.sql"))?;
        statement.execute(params![self.id, self.name, self.order, self.generation])
    }
}
impl SelectAllNames for GameRow {}

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
    fn label() -> &'static str {
        "Move"
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
    fn insert(&self, db: &Connection) -> SqlResult<usize> {
        let mut statement = db.prepare_cached(include_str!("../sql/insert_move.sql"))?;
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
impl SelectAllNames for MoveRow {}

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
    fn label() -> &'static str {
        "Move Change"
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
    fn insert(&self, db: &Connection) -> SqlResult<usize> {
        let mut statement = db.prepare_cached(include_str!("../sql/insert_move_change.sql"))?;
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

pub enum MoveRowGroup {
    MoveRow(MoveRow),
    MoveChangeRow(MoveChangeRow),
}
impl InsertRow for MoveRowGroup {
    fn insert(&self, db: &Connection) -> SqlResult<usize> {
        match self {
            MoveRowGroup::MoveChangeRow(row) => row.insert(db),
            MoveRowGroup::MoveRow(row) => row.insert(db),
        }
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
    fn label() -> &'static str {
        "Type"
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
    fn insert(&self, db: &Connection) -> SqlResult<usize> {
        let mut statement = db.prepare_cached(include_str!("../sql/insert_type.sql"))?;
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
impl SelectAllNames for TypeRow {}

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
    fn label() -> &'static str {
        "Type Change"
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
    fn insert(&self, db: &Connection) -> SqlResult<usize> {
        let mut statement = db.prepare_cached(include_str!("../sql/insert_type_change.sql"))?;
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

pub enum TypeRowGroup {
    TypeRow(TypeRow),
    TypeChangeRow(TypeChangeRow),
}
impl InsertRow for TypeRowGroup {
    fn insert(&self, db: &Connection) -> SqlResult<usize> {
        match self {
            TypeRowGroup::TypeRow(row) => row.insert(db),
            TypeRowGroup::TypeChangeRow(row) => row.insert(db),
        }
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
    fn label() -> &'static str {
        "Ability"
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
    fn insert(&self, db: &Connection) -> SqlResult<usize> {
        let mut statement = db.prepare_cached(include_str!("../sql/insert_ability.sql"))?;
        statement.execute(params![self.id, self.name, self.effect, self.generation])
    }
}
impl SelectAllNames for AbilityRow {}

pub struct EvolutionRow {
    pub id: i64,
    pub evolution: String,
}
impl TableRow for EvolutionRow {
    fn table() -> &'static str {
        "evolutions"
    }
    fn label() -> &'static str {
        "Evolution"
    }
}
impl InsertRow for EvolutionRow {
    fn insert(&self, db: &Connection) -> SqlResult<usize> {
        let mut statement = db.prepare_cached(include_str!("../sql/insert_evolution.sql"))?;
        statement.execute(params![self.id, self.evolution,])
    }
}
impl SelectRow for EvolutionRow {
    fn on_hit(row: &Row<'_>) -> SqlResult<Self> {
        Ok(Self {
            id: row.get(0)?,
            evolution: row.get(1)?,
        })
    }
}

pub struct SpeciesRow {
    pub id: i64,
    pub name: String,
    pub is_baby: bool,
    pub is_legendary: bool,
    pub is_mythical: bool,
    pub evolution_id: Option<i64>,
}
impl TableRow for SpeciesRow {
    fn table() -> &'static str {
        "species"
    }
    fn label() -> &'static str {
        "Species"
    }
}
impl InsertRow for SpeciesRow {
    fn insert(&self, db: &Connection) -> SqlResult<usize> {
        let mut statement = db.prepare_cached(include_str!("../sql/insert_species.sql"))?;
        statement.execute(params![
            self.id,
            self.name,
            self.is_baby,
            self.is_legendary,
            self.is_mythical,
            self.evolution_id,
        ])
    }
}
impl SelectRow for SpeciesRow {
    fn on_hit(row: &Row<'_>) -> SqlResult<Self> {
        Ok(Self {
            id: row.get(0)?,
            name: row.get(1)?,
            is_baby: row.get(2)?,
            is_legendary: row.get(3)?,
            is_mythical: row.get(4)?,
            evolution_id: row.get(5)?,
        })
    }
}

pub struct PokemonRow {
    pub id: i64,
    pub name: String,
    pub primary_type: String,
    pub secondary_type: Option<String>,
    pub hp: i64,
    pub attack: i64,
    pub defense: i64,
    pub special_attack: i64,
    pub special_defense: i64,
    pub speed: i64,
    pub species_id: i64,
}
impl TableRow for PokemonRow {
    fn table() -> &'static str {
        "pokemon"
    }
    fn label() -> &'static str {
        "Pokémon"
    }
}
impl InsertRow for PokemonRow {
    fn insert(&self, db: &Connection) -> SqlResult<usize> {
        let mut statement = db.prepare_cached(include_str!("../sql/insert_pokemon.sql"))?;
        statement.execute(params![
            self.id,
            self.name,
            self.primary_type,
            self.secondary_type,
            self.hp,
            self.attack,
            self.defense,
            self.special_attack,
            self.special_defense,
            self.speed,
            self.species_id,
        ])
    }
}
impl SelectRow for PokemonRow {
    fn on_hit(row: &Row<'_>) -> SqlResult<Self> {
        Ok(Self {
            id: row.get(0)?,
            name: row.get(1)?,
            primary_type: row.get(2)?,
            secondary_type: row.get(3)?,
            hp: row.get(4)?,
            attack: row.get(5)?,
            defense: row.get(6)?,
            special_attack: row.get(7)?,
            special_defense: row.get(8)?,
            speed: row.get(9)?,
            species_id: row.get(10)?,
        })
    }
}
impl SelectAllNames for PokemonRow {}

pub struct PokemonMoveRow {
    pub id: Option<i64>,
    pub move_id: i64,
    pub learn_method: String,
    pub learn_level: i64,
    pub generation: u8,
    pub pokemon_id: i64,
}
impl TableRow for PokemonMoveRow {
    fn table() -> &'static str {
        "pokemon_moves"
    }
    fn label() -> &'static str {
        "Pokémon Move"
    }
}
impl InsertRow for PokemonMoveRow {
    fn insert(&self, db: &Connection) -> SqlResult<usize> {
        let mut statement = db.prepare_cached(include_str!("../sql/insert_pokemon_move.sql"))?;
        statement.execute(params![
            self.id,
            self.move_id,
            self.learn_method,
            self.learn_level,
            self.generation,
            self.pokemon_id,
        ])
    }
}
impl PokemonMoveRow {
    pub fn select_by_pokemon(
        pokemon_id: i64,
        generation: u8,
        db: &Connection,
    ) -> SqlResult<Vec<(String, String, i64)>> {
        let mut statement = db.prepare_cached(include_str!("../sql/select_pokemon_moves.sql"))?;
        let rows = statement.query_map([pokemon_id, generation as i64], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?))
        })?;

        let mut moves = vec![];
        for row in rows {
            let row = row?;
            moves.push((row.0, row.1, row.2));
        }

        Ok(moves)
    }
}

pub struct PokemonAbilityRow {
    pub id: Option<i64>,
    pub ability_id: i64,
    pub is_hidden: bool,
    pub slot: i64,
    pub pokemon_id: i64,
}
impl TableRow for PokemonAbilityRow {
    fn table() -> &'static str {
        "pokemon_abilities"
    }
    fn label() -> &'static str {
        "Pokémon Ability"
    }
}
impl InsertRow for PokemonAbilityRow {
    fn insert(&self, db: &Connection) -> SqlResult<usize> {
        let mut statement = db.prepare_cached(include_str!("../sql/insert_pokemon_ability.sql"))?;
        statement.execute(params![
            self.id,
            self.ability_id,
            self.is_hidden,
            self.slot,
            self.pokemon_id,
        ])
    }
}
impl PokemonAbilityRow {
    pub fn select_by_pokemon(pokemon_id: i64, db: &Connection) -> SqlResult<Vec<(String, bool)>> {
        let mut statement =
            db.prepare_cached(include_str!("../sql/select_pokemon_abilities.sql"))?;
        let rows = statement.query_map([pokemon_id], |row| Ok((row.get(0)?, row.get(1)?)))?;

        let mut abilities = vec![];
        for row in rows {
            abilities.push(row?);
        }

        Ok(abilities)
    }
}

pub struct PokemonTypeChangeRow {
    pub id: Option<i64>,
    pub primary_type: String,
    pub secondary_type: Option<String>,
    pub generation: u8,
    pub pokemon_id: i64,
}
impl TableRow for PokemonTypeChangeRow {
    fn table() -> &'static str {
        "pokemon_type_changes"
    }
    fn label() -> &'static str {
        "Pokémon Type Change"
    }
}
impl InsertRow for PokemonTypeChangeRow {
    fn insert(&self, db: &Connection) -> SqlResult<usize> {
        let mut statement =
            db.prepare_cached(include_str!("../sql/insert_pokemon_type_change.sql"))?;
        statement.execute(params![
            self.id,
            self.primary_type,
            self.secondary_type,
            self.generation,
            self.pokemon_id,
        ])
    }
}
impl SelectChangeRow for PokemonTypeChangeRow {
    fn fk() -> &'static str {
        "pokemon_id"
    }

    fn on_hit(row: &Row<'_>) -> SqlResult<Self> {
        Ok(Self {
            id: row.get(0)?,
            primary_type: row.get(1)?,
            secondary_type: row.get(2)?,
            generation: row.get(3)?,
            pokemon_id: row.get(4)?,
        })
    }
}

pub enum PokemonRowGroup {
    PokemonRow(PokemonRow),
    PokemonMoveRow(PokemonMoveRow),
    PokemonAbilityRow(PokemonAbilityRow),
    PokemonTypeChangeRow(PokemonTypeChangeRow),
}
impl InsertRow for PokemonRowGroup {
    fn insert(&self, db: &Connection) -> SqlResult<usize> {
        match self {
            PokemonRowGroup::PokemonRow(row) => row.insert(db),
            PokemonRowGroup::PokemonMoveRow(row) => row.insert(db),
            PokemonRowGroup::PokemonAbilityRow(row) => row.insert(db),
            PokemonRowGroup::PokemonTypeChangeRow(row) => row.insert(db),
        }
    }
}

pub struct MetaRow {
    pub name: String,
    pub value: String,
}
impl TableRow for MetaRow {
    fn table() -> &'static str {
        "meta"
    }
    fn label() -> &'static str {
        "Meta"
    }
}
impl InsertRow for MetaRow {
    fn insert(&self, db: &Connection) -> SqlResult<usize> {
        let mut statement = db.prepare_cached(include_str!("../sql/insert_meta.sql"))?;
        statement.execute(params![self.name, self.value])
    }
}
impl SelectRow for MetaRow {
    fn on_hit(row: &Row<'_>) -> SqlResult<Self> {
        Ok(Self {
            name: row.get(0)?,
            value: row.get(1)?,
        })
    }
}

pub trait Validate<T> {
    fn validate(&self, value: &str) -> Result<String> {
        let value = value.to_lowercase();
        match self.check(&value) {
            ResourceResult::Valid => Ok(value),
            ResourceResult::Invalid(matches) => bail!(Self::invalid_message(&value, &matches)),
        }
    }

    fn check(&self, value: &str) -> ResourceResult {
        let matches = self.get_matches(value);
        if matches.iter().any(|m| *m == value) {
            ResourceResult::Valid
        } else {
            ResourceResult::Invalid(matches)
        }
    }

    fn get_matches(&self, value: &str) -> Vec<String> {
        self.get_resource()
            .iter()
            .filter_map(|r| {
                let close_enough = if !r.is_empty() && !value.is_empty() {
                    let first_r = r.chars().next().unwrap();
                    let first_value = value.chars().next().unwrap();

                    // Only perform spellcheck on first character match; potentially expensive
                    first_r == first_value && strsim::levenshtein(r, value) < 4
                } else {
                    false
                };

                if r.contains(value) || close_enough {
                    Some(r.clone())
                } else {
                    None
                }
            })
            .collect::<Vec<String>>()
    }

    fn invalid_message(value: &str, matches: &[String]) -> String {
        let resource_name = Self::label();
        let mut message = format!("{resource_name} '{value}' not found.");

        if matches.len() > 20 {
            message += " Potential matches found; too many to display.";
        } else if !matches.is_empty() {
            message += &format!(" Potential matches: {}.", matches.join(" "));
        }

        message
    }

    fn get_resource(&self) -> Vec<String>;
    fn label() -> &'static str;
}

impl<T: SelectAllNames> Validate<T> for Connection {
    fn get_resource(&self) -> Vec<String> {
        T::select_all_names(self).unwrap()
    }

    fn label() -> &'static str {
        T::label()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockRow;
    impl TableRow for MockRow {
        fn table() -> &'static str {
            "mock_row"
        }
        fn label() -> &'static str {
            "Row"
        }
    }
    impl SelectAllNames for MockRow {}

    struct MockResource;
    impl Validate<MockRow> for MockResource {
        fn get_resource(&self) -> Vec<String> {
            vec!["orangutan", "cricket", "ocelot", "toucan", "wendigo"]
                .into_iter()
                .map(String::from)
                .collect()
        }

        fn label() -> &'static str {
            MockRow::label()
        }
    }

    #[test]
    fn resource_validates() {
        let resource = MockResource;

        let err = resource
            .validate("osselot")
            .expect_err("ocelot should only be a potential match via levenshtein distance");
        assert_eq!(
            String::from("Row 'osselot' not found. Potential matches: ocelot."),
            err.to_string()
        );

        let err = resource
            .validate("toucannon")
            .expect_err("toucannon should only be a potential match via substring");
        assert_eq!(
            String::from("Row 'toucannon' not found. Potential matches: toucan."),
            err.to_string()
        );

        let ok = resource
            .validate("cricket")
            .expect("cricket should be a valid");
        assert_eq!(String::from("cricket"), ok);

        let ok = resource
            .validate("Wendigo")
            .expect("Wendigo should be valid; validate is case-insensitive");
        assert_eq!(String::from("wendigo"), ok);
    }
}
