pub mod resource;

use resource::{
    AbilityRow, EvolutionRow, FromRow, GameRow, MoveChangeRow, MoveRow, PokemonAbilityRow,
    PokemonMoveRow, PokemonRow, PokemonTypeChangeRow, SelectChangeRow, SelectRow, SpeciesRow,
    TypeChangeRow, TypeRow,
};

use std::collections::HashMap;
use std::ops::Add;

use anyhow::{bail, Result};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};

pub struct Pokemon {
    pub data: PokemonData,
    pub defense_chart: DefenseTypeChart,
    pub move_list: MoveList,
}

impl Pokemon {
    pub fn new(data: PokemonData, defense_chart: DefenseTypeChart, move_list: MoveList) -> Self {
        Self {
            data,
            defense_chart,
            move_list,
        }
    }
}

#[derive(Debug)]
pub struct PokemonData {
    pub name: String,
    pub primary_type: String,
    pub secondary_type: Option<String>,
    pub learn_moves: Vec<(String, String, i64)>,
    pub group: PokemonGroup,
    pub generation: u8,
    pub stats: Stats,
    pub abilities: Vec<(String, bool)>,
    pub species: String,
}
impl PokemonData {
    pub fn from_name(pokemon_name: &str, generation: u8, db: &Connection) -> Result<Self> {
        let pokemon_row = PokemonRow::select_by_name(pokemon_name, db)?;
        PokemonData::from_row(pokemon_row, generation, db)
    }

    pub fn get_moves(&self, db: &Connection) -> Result<MoveList> {
        let moves_results = self
            .learn_moves
            .iter()
            .map(|mv| Move::from_name(&mv.0, self.generation, db))
            .collect::<Vec<Result<Move>>>();

        let mut moves = HashMap::new();
        for move_ in moves_results {
            let move_ = move_?;
            moves.insert(move_.name.clone(), move_);
        }

        Ok(MoveList::new(moves))
    }

    pub fn get_defense_chart(&self, db: &Connection) -> Result<DefenseTypeChart> {
        let primary_type = Type::from_name(&self.primary_type, self.generation, db)?;

        if let Some(secondary_type) = &self.secondary_type {
            let secondary_type = Type::from_name(secondary_type, self.generation, db)?;

            Ok(primary_type.defense_chart + secondary_type.defense_chart)
        } else {
            Ok(primary_type.defense_chart)
        }
    }

    pub fn get_evolution_steps(&self, db: &Connection) -> Result<EvolutionStep> {
        let species_row = SpeciesRow::select_by_name(&self.species, db)?;
        let evolution_row = EvolutionRow::select_by_id(species_row.evolution_id.unwrap(), db)?;
        Ok(serde_json::from_str(&evolution_row.evolution)?)
    }
}
impl FromRow<PokemonRow> for PokemonData {
    fn from_row(value: PokemonRow, current_gen: u8, db: &Connection) -> Result<Self> {
        let PokemonRow {
            id,
            name,
            mut primary_type,
            mut secondary_type,
            hp,
            attack,
            defense,
            special_attack,
            special_defense,
            speed,
            species_id,
        } = value;

        let learn_moves = PokemonMoveRow::select_by_pokemon(id, current_gen, db)?;
        // PokéAPI keeps a pokemon's release generation in the 'species' resource.
        // However, the proliferation of various pokemon forms has rendered that value inaccurate.
        // A better way is to check if they have any moves in the specified generation.
        if learn_moves.is_empty() {
            bail!(format!(
                "Pokémon '{name}' is not present in generation {current_gen}"
            ));
        }

        let stats = Stats {
            hp,
            attack,
            defense,
            special_attack,
            special_defense,
            speed,
        };

        let type_change_row = PokemonTypeChangeRow::select_by_fk(id, current_gen, db)?;
        if let Some(change) = type_change_row {
            primary_type = change.primary_type;
            secondary_type = change.secondary_type;
        }

        let species_row = SpeciesRow::select_by_id(species_id, db)?;
        let species = species_row.name.clone();
        let group = PokemonGroup::from(species_row);

        let abilities = PokemonAbilityRow::select_by_pokemon(id, db)?;

        Ok(Self {
            name,
            primary_type,
            secondary_type,
            learn_moves,
            group,
            generation: current_gen,
            stats,
            abilities,
            species,
        })
    }
}

#[derive(Debug)]
pub enum PokemonGroup {
    Mythical,
    Legendary,
    Regular,
    Baby,
}
impl std::fmt::Display for PokemonGroup {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let str = match self {
            PokemonGroup::Mythical => "mythical",
            PokemonGroup::Legendary => "legendary",
            PokemonGroup::Baby => "legendary",
            PokemonGroup::Regular => "",
        };

        write!(f, "{str}")
    }
}
impl From<SpeciesRow> for PokemonGroup {
    fn from(value: SpeciesRow) -> Self {
        if value.is_mythical {
            Self::Mythical
        } else if value.is_legendary {
            Self::Legendary
        } else if value.is_baby {
            Self::Baby
        } else {
            Self::Regular
        }
    }
}

#[derive(Default, Debug)]
pub struct Stats {
    pub hp: i64,
    pub attack: i64,
    pub defense: i64,
    pub special_attack: i64,
    pub special_defense: i64,
    pub speed: i64,
}

#[derive(Debug)]
pub struct Type {
    pub name: String,
    pub offense_chart: OffenseTypeChart,
    pub defense_chart: DefenseTypeChart,
    pub generation: u8,
}
impl Type {
    pub fn from_name(type_name: &str, generation: u8, db: &Connection) -> Result<Self> {
        let type_row = TypeRow::select_by_name(type_name, db)?;
        Type::from_row(type_row, generation, db)
    }

    fn relation_to_hashmap(
        no_damage: &str,
        half_damage: &str,
        double_damage: &str,
    ) -> HashMap<String, f32> {
        let mut chart = HashMap::new();

        Self::split_and_insert(&mut chart, no_damage, 0.0);
        Self::split_and_insert(&mut chart, half_damage, 0.5);
        Self::split_and_insert(&mut chart, double_damage, 2.0);

        chart
    }

    fn split_and_insert(chart: &mut HashMap<String, f32>, damage_relation: &str, value: f32) {
        damage_relation
            .split(',')
            .collect::<Vec<&str>>()
            .into_iter()
            .for_each(|type_| {
                if !type_.is_empty() {
                    chart.insert(type_.to_string(), value);
                }
            });
    }
}
impl FromRow<TypeRow> for Type {
    fn from_row(value: TypeRow, current_gen: u8, db: &Connection) -> Result<Self> {
        let TypeRow {
            id,
            name,
            mut no_damage_to,
            mut half_damage_to,
            mut double_damage_to,
            mut no_damage_from,
            mut half_damage_from,
            mut double_damage_from,
            generation,
        } = value;

        if current_gen < generation {
            bail!(format!(
                "Type '{name}' is not present in generation {current_gen}"
            ));
        }

        let change_row = TypeChangeRow::select_by_fk(id, current_gen, db)?;
        if let Some(change) = change_row {
            no_damage_to = change.no_damage_to;
            half_damage_to = change.half_damage_to;
            double_damage_to = change.double_damage_to;

            no_damage_from = change.no_damage_from;
            half_damage_from = change.half_damage_from;
            double_damage_from = change.double_damage_from;
        }

        let mut offense_chart = OffenseTypeChart::new(Self::relation_to_hashmap(
            &no_damage_to,
            &half_damage_to,
            &double_damage_to,
        ));
        offense_chart.set_label(&name);

        let mut defense_chart = DefenseTypeChart::new(Self::relation_to_hashmap(
            &no_damage_from,
            &half_damage_from,
            &double_damage_from,
        ));
        defense_chart.set_label(&name);

        Ok(Self {
            name,
            offense_chart,
            defense_chart,
            generation,
        })
    }
}

pub const TYPES: [&str; 19] = [
    "normal", "fighting", "fire", "fighting", "water", "flying", "grass", "poison", "electric",
    "ground", "psychic", "rock", "ice", "bug", "dragon", "ghost", "dark", "steel", "fairy",
];

fn default_chart() -> HashMap<String, f32> {
    let mut chart = HashMap::new();

    for type_ in TYPES {
        chart.insert(type_.to_string(), 1.0f32);
    }

    chart
}

fn combine_charts(
    chart1: &HashMap<String, f32>,
    chart2: &HashMap<String, f32>,
) -> HashMap<String, f32> {
    let mut new_chart = HashMap::new();

    for (type_, multiplier) in chart1 {
        new_chart.insert(type_.clone(), *multiplier);
    }

    for (type_, multiplier) in chart2 {
        if let Some(new_multiplier) = new_chart.get(type_) {
            new_chart.insert(type_.clone(), multiplier * new_multiplier);
        } else {
            new_chart.insert(type_.clone(), *multiplier);
        }
    }

    new_chart
}

pub trait TypeChart {
    fn get_multiplier(&self, type_: &str) -> f32 {
        *self.get_chart().get(type_).unwrap()
    }

    fn get_chart(&self) -> &HashMap<String, f32>;
    fn get_type(&self) -> TypeCharts;
    fn get_label(&self) -> String;
    fn set_label(&mut self, label: &str);
}

pub enum TypeCharts {
    Offense,
    Defense,
}

pub trait NewTypeChart: Sized {
    fn new(chart: HashMap<String, f32>) -> Self {
        let default = default_chart();
        let new_chart = combine_charts(&default, &chart);
        Self::new_struct(new_chart)
    }

    fn new_struct(chart: HashMap<String, f32>) -> Self;
}

#[derive(Debug)]
pub struct OffenseTypeChart {
    chart: HashMap<String, f32>,
    label: String,
}
impl NewTypeChart for OffenseTypeChart {
    fn new_struct(chart: HashMap<String, f32>) -> Self {
        Self {
            chart,
            label: String::from(""),
        }
    }
}
impl TypeChart for OffenseTypeChart {
    fn get_chart(&self) -> &HashMap<String, f32> {
        &self.chart
    }

    fn get_type(&self) -> TypeCharts {
        TypeCharts::Offense
    }

    fn get_label(&self) -> String {
        self.label.clone()
    }

    fn set_label(&mut self, label: &str) {
        self.label = String::from(label);
    }
}

#[derive(Debug)]
pub struct DefenseTypeChart {
    chart: HashMap<String, f32>,
    label: String,
}
impl NewTypeChart for DefenseTypeChart {
    fn new_struct(chart: HashMap<String, f32>) -> Self {
        Self {
            chart,
            label: String::from(""),
        }
    }
}
impl TypeChart for DefenseTypeChart {
    fn get_chart(&self) -> &HashMap<String, f32> {
        &self.chart
    }

    fn get_type(&self) -> TypeCharts {
        TypeCharts::Defense
    }

    fn get_label(&self) -> String {
        self.label.clone()
    }

    fn set_label(&mut self, label: &str) {
        self.label = String::from(label);
    }
}
impl Add for DefenseTypeChart {
    type Output = DefenseTypeChart;
    fn add(self, rhs: Self) -> Self::Output {
        let chart = combine_charts(self.get_chart(), rhs.get_chart());
        let label = self.label + " " + &rhs.label;
        Self { chart, label }
    }
}

#[derive(Debug)]
pub struct Move {
    pub name: String,
    pub accuracy: Option<i64>,
    pub power: Option<i64>,
    pub pp: Option<i64>,
    pub damage_class: String,
    pub type_: String,
    pub effect: String,
    pub effect_chance: Option<i64>,
    pub generation: u8,
}
impl Move {
    pub fn from_name(move_name: &str, generation: u8, db: &Connection) -> Result<Self> {
        let move_row = MoveRow::select_by_name(move_name, db)?;
        Move::from_row(move_row, generation, db)
    }
}
impl FromRow<MoveRow> for Move {
    fn from_row(value: MoveRow, current_gen: u8, db: &Connection) -> Result<Self> {
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

        if current_gen < generation {
            bail!(format!(
                "Move '{name}' is not present in generation {current_gen}"
            ));
        }

        let change_row = MoveChangeRow::select_by_fk(id, current_gen, db)?;
        if let Some(change) = change_row {
            power = change.power.or(power);
            accuracy = change.accuracy.or(accuracy);
            pp = change.pp.or(pp);
            effect_chance = change.effect_chance.or(effect_chance);

            if let Some(t) = change.type_ {
                type_ = t;
            }
        }

        Ok(Self {
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

pub struct MoveList {
    value: HashMap<String, Move>,
}

impl MoveList {
    pub fn new(hashmap: HashMap<String, Move>) -> MoveList {
        MoveList { value: hashmap }
    }

    pub fn get_map(&self) -> &HashMap<String, Move> {
        &self.value
    }
}

#[derive(Debug)]
pub struct Ability {
    pub name: String,
    pub effect: String,
    pub generation: u8,
}
impl Ability {
    pub fn from_name(ability_name: &str, generation: u8, db: &Connection) -> Result<Self> {
        let ability_row = AbilityRow::select_by_name(ability_name, db)?;
        Ability::from_row(ability_row, generation, db)
    }
}
impl FromRow<AbilityRow> for Ability {
    fn from_row(value: AbilityRow, current_gen: u8, _db: &Connection) -> Result<Self> {
        let AbilityRow {
            name,
            effect,
            generation,
            ..
        } = value;

        if current_gen < generation {
            bail!(format!(
                "Ability '{name}' is not present in generation {current_gen}"
            ));
        }

        Ok(Self {
            name,
            effect,
            generation,
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EvolutionStep {
    pub name: String,
    pub methods: Vec<EvolutionMethod>,
    pub evolves_to: Vec<EvolutionStep>,
}

impl EvolutionStep {
    pub fn new(
        name: String,
        methods: Vec<EvolutionMethod>,
        evolves_to: Vec<EvolutionStep>,
    ) -> Self {
        Self {
            name,
            methods,
            evolves_to,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EvolutionMethod {
    pub trigger: String,
    pub item: Option<String>,
    pub gender: Option<i64>,
    pub held_item: Option<String>,
    pub known_move: Option<String>,
    pub known_move_type: Option<String>,
    pub location: Option<String>,
    pub min_level: Option<i64>,
    pub min_happiness: Option<i64>,
    pub min_beauty: Option<i64>,
    pub min_affection: Option<i64>,
    pub needs_overworld_rain: Option<bool>,
    pub party_species: Option<String>,
    pub party_type: Option<String>,
    pub relative_physical_stats: Option<i64>,
    pub time_of_day: Option<String>,
    pub trade_species: Option<String>,
    pub turn_upside_down: Option<bool>,
}

impl EvolutionMethod {
    pub fn new(trigger: String) -> Self {
        Self {
            trigger,
            item: None,
            gender: None,
            held_item: None,
            known_move: None,
            known_move_type: None,
            location: None,
            min_level: None,
            min_happiness: None,
            min_beauty: None,
            min_affection: None,
            needs_overworld_rain: None,
            party_species: None,
            party_type: None,
            relative_physical_stats: None,
            time_of_day: None,
            trade_species: None,
            turn_upside_down: None,
        }
    }

    pub fn item(mut self, item: String) -> Self {
        self.item = Some(item);
        self
    }

    pub fn gender(mut self, gender: i64) -> Self {
        self.gender = Some(gender);
        self
    }

    pub fn held_item(mut self, held_item: String) -> Self {
        self.held_item = Some(held_item);
        self
    }

    pub fn known_move(mut self, known_move: String) -> Self {
        self.known_move = Some(known_move);
        self
    }

    pub fn known_move_type(mut self, known_move_type: String) -> Self {
        self.known_move_type = Some(known_move_type);
        self
    }

    pub fn location(mut self, location: String) -> Self {
        self.location = Some(location);
        self
    }

    pub fn min_level(mut self, min_level: i64) -> Self {
        self.min_level = Some(min_level);
        self
    }

    pub fn min_happiness(mut self, min_happiness: i64) -> Self {
        self.min_happiness = Some(min_happiness);
        self
    }

    pub fn min_beauty(mut self, min_beauty: i64) -> Self {
        self.min_beauty = Some(min_beauty);
        self
    }

    pub fn min_affection(mut self, min_affection: i64) -> Self {
        self.min_affection = Some(min_affection);
        self
    }

    pub fn needs_overworld_rain(mut self, needs_overworld_rain: bool) -> Self {
        self.needs_overworld_rain = Some(needs_overworld_rain);
        self
    }

    pub fn party_species(mut self, party_species: String) -> Self {
        self.party_species = Some(party_species);
        self
    }

    pub fn party_type(mut self, party_type: String) -> Self {
        self.party_type = Some(party_type);
        self
    }

    pub fn relative_physical_stats(mut self, relative_physical_stats: i64) -> Self {
        self.relative_physical_stats = Some(relative_physical_stats);
        self
    }

    pub fn time_of_day(mut self, time_of_day: String) -> Self {
        self.time_of_day = Some(time_of_day);
        self
    }

    pub fn trade_species(mut self, trade_species: String) -> Self {
        self.trade_species = Some(trade_species);
        self
    }

    pub fn turn_upside_down(mut self, turn_upside_down: bool) -> Self {
        self.turn_upside_down = Some(turn_upside_down);
        self
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Game {
    pub name: String,
    pub order: u8,
    pub generation: u8,
}
impl Game {
    pub fn new(name: String, order: u8, generation: u8) -> Self {
        Self {
            name,
            order,
            generation,
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resource::database::DatabaseFile;

    fn db() -> Connection {
        let db_file = DatabaseFile::default();
        db_file.connect().unwrap()
    }

    #[test]
    fn get_pokemon_by_name() {
        let db = db();

        // Ogerpon was not inroduced until gen 9
        PokemonData::from_name("ogerpon", 8, &db).unwrap_err();
        PokemonData::from_name("ogerpon", 9, &db).unwrap();

        // Wailord is not present in gen 9, but is present in gen 8
        PokemonData::from_name("wailord", 9, &db).unwrap_err();
        PokemonData::from_name("wailord", 8, &db).unwrap();

        // Test dual type defense chart
        let golem = PokemonData::from_name("golem", 9, &db).unwrap();
        let golem_defense = golem.get_defense_chart(&db).unwrap();
        assert_eq!(4.0, golem_defense.get_multiplier("water"));
        assert_eq!(2.0, golem_defense.get_multiplier("fighting"));
        assert_eq!(1.0, golem_defense.get_multiplier("psychic"));
        assert_eq!(0.5, golem_defense.get_multiplier("flying"));
        assert_eq!(0.25, golem_defense.get_multiplier("poison"));
        assert_eq!(0.0, golem_defense.get_multiplier("electric"));

        // Clefairy was Normal type until gen 6
        let clefairy_gen_5 = PokemonData::from_name("clefairy", 5, &db).unwrap();
        assert_eq!("normal", clefairy_gen_5.primary_type);
        let clefairy_gen_6 = PokemonData::from_name("clefairy", 6, &db).unwrap();
        assert_eq!("fairy", clefairy_gen_6.primary_type);
    }

    #[test]
    fn get_pokemon_evolution() {
        let db = db();

        let cascoon = PokemonData::from_name("cascoon", 3, &db)
            .unwrap()
            .get_evolution_steps(&db)
            .unwrap();
        insta::assert_yaml_snapshot!(cascoon);

        let applin = PokemonData::from_name("applin", 9, &db)
            .unwrap()
            .get_evolution_steps(&db)
            .unwrap();
        insta::assert_yaml_snapshot!(applin);

        let politoed = PokemonData::from_name("politoed", 9, &db)
            .unwrap()
            .get_evolution_steps(&db)
            .unwrap();
        insta::assert_yaml_snapshot!(politoed);
    }

    #[test]
    fn get_type_by_name() {
        let db = db();

        // Fairy was not introduced until gen 6
        Type::from_name("fairy", 5, &db).unwrap_err();
        Type::from_name("fairy", 6, &db).unwrap();

        // Bug gen 1 2x against poison
        let bug_gen_1 = Type::from_name("bug", 1, &db).unwrap();
        assert_eq!(2.0, bug_gen_1.offense_chart.get_multiplier("poison"));
        assert_eq!(1.0, bug_gen_1.offense_chart.get_multiplier("dark"));

        // Bug gen >=2 2x against dark
        let bug_gen_2 = Type::from_name("bug", 2, &db).unwrap();
        assert_eq!(0.5, bug_gen_2.offense_chart.get_multiplier("poison"));
        assert_eq!(2.0, bug_gen_2.offense_chart.get_multiplier("dark"));
    }

    #[test]
    fn get_move_by_name() {
        let db = db();

        // Earth Power was not introduced until gen 4
        Move::from_name("earth-power", 3, &db).unwrap_err();
        Move::from_name("earth-power", 4, &db).unwrap();

        // Tackle gen 1-4 power: 35 accuracy: 95
        let tackle_gen_4 = Move::from_name("tackle", 4, &db).unwrap();
        assert_eq!(35, tackle_gen_4.power.unwrap());
        assert_eq!(95, tackle_gen_4.accuracy.unwrap());

        // Tackle gen 5-6 power: 50 accuracy: 100
        let tackle_gen_5 = Move::from_name("tackle", 5, &db).unwrap();
        assert_eq!(50, tackle_gen_5.power.unwrap());
        assert_eq!(100, tackle_gen_5.accuracy.unwrap());

        // Tackle gen >=7 power: 40 accuracy: 100
        let tackle_gen_7 = Move::from_name("tackle", 7, &db).unwrap();
        assert_eq!(40, tackle_gen_7.power.unwrap());
        assert_eq!(100, tackle_gen_7.accuracy.unwrap());
    }

    #[test]
    fn get_ability_by_name() {
        let db = db();

        // Beads of Ruin was not introduced until gen 9
        Ability::from_name("beads-of-ruin", 8, &db).unwrap_err();
        Ability::from_name("beads-of-ruin", 9, &db).unwrap();
    }

    #[test]
    fn combine_charts_test() {
        let mut chart1 = HashMap::new();
        chart1.insert("fire".to_string(), 2.0);
        chart1.insert("water".to_string(), 0.5);
        chart1.insert("steel".to_string(), 0.0);

        let mut chart2 = HashMap::new();
        chart2.insert("fire".to_string(), 2.0);
        chart2.insert("water".to_string(), 1.0);
        chart2.insert("ice".to_string(), 1.0);

        let combined = combine_charts(&chart1, &chart2);

        assert_eq!(combined.get("fire"), Some(&4.0));
        assert_eq!(combined.get("water"), Some(&0.5));
        assert_eq!(combined.get("steel"), Some(&0.0));
        assert_eq!(combined.get("ice"), Some(&1.0));
    }
}
