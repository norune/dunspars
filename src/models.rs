pub mod resource;

use crate::api;

use std::collections::HashMap;
use std::ops::Add;

use anyhow::Result;
use futures::stream::FuturesUnordered;
use futures::StreamExt;
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
    pub learn_moves: HashMap<String, (String, i64)>,
    pub group: PokemonGroup,
    pub game: String,
    pub generation: u8,
    pub stats: Stats,
    pub abilities: Vec<(String, bool)>,
    pub species: String,
}

impl PokemonData {
    pub async fn from_name(name: &str, game: &str) -> Result<Self> {
        api::get_pokemon(name, game).await
    }

    pub async fn get_moves(&self) -> Result<MoveList> {
        let moves_futures: FuturesUnordered<_> = self
            .learn_moves
            .iter()
            .map(|mv| Move::from_name(mv.0, self.generation))
            .collect();
        let moves_results: Vec<_> = moves_futures.collect().await;

        let mut moves = HashMap::new();
        for result in moves_results {
            let move_ = result?;
            moves.insert(move_.name.clone(), move_);
        }

        Ok(MoveList::new(moves))
    }

    pub async fn get_defense_chart(&self) -> Result<DefenseTypeChart> {
        let primary_type = Type::from_name(&self.primary_type, self.generation).await?;

        if let Some(secondary_type) = &self.secondary_type {
            let secondary_type = Type::from_name(secondary_type, self.generation).await?;

            Ok(primary_type.defense_chart + secondary_type.defense_chart)
        } else {
            Ok(primary_type.defense_chart)
        }
    }

    pub async fn get_evolution_steps(&self) -> Result<EvolutionStep> {
        api::get_evolution(&self.species).await
    }
}

#[derive(Debug)]
pub enum PokemonGroup {
    Mythical,
    Legendary,
    Regular,
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
    pub async fn from_name(name: &str, generation: u8) -> Result<Self> {
        api::get_type(name, generation).await
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
    pub async fn from_name(move_name: &str, generation: u8) -> Result<Self> {
        api::get_move(move_name, generation).await
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
    pub short_effect: String,
    pub generation: u8,
}

impl Ability {
    pub async fn from_name(name: &str, generation: u8) -> Result<Self> {
        api::get_ability(name, generation).await
    }
}

#[derive(Debug, Serialize)]
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

#[derive(Debug, Serialize)]
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

#[cfg(test)]
mod tests {
    use super::*;

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
