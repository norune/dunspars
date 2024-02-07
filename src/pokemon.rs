use std::collections::HashMap;

use anyhow::{bail, Result};
use futures::stream::FuturesUnordered;
use futures::StreamExt;
use strsim;

use crate::api::ApiWrapper;

pub struct Pokemon<'a> {
    pub data: PokemonData<'a>,
    pub defense_chart: TypeChart,
    pub move_list: MoveList<'a>,
}

impl<'a> Pokemon<'a> {
    pub fn new(data: PokemonData<'a>, defense_chart: TypeChart, move_list: MoveList<'a>) -> Self {
        Self {
            data,
            defense_chart,
            move_list,
        }
    }
}

pub struct PokemonData<'a> {
    pub name: String,
    pub primary_type: String,
    pub secondary_type: Option<String>,
    pub learn_moves: HashMap<String, (String, i64)>,
    pub game: String,
    pub generation: u8,
    pub stats: Stats,
    pub abilities: Vec<(String, bool)>,
    pub species: String,
    pub api: &'a ApiWrapper,
}

impl<'a> PokemonData<'a> {
    pub async fn from_name(api: &'a ApiWrapper, name: &str, game: &str) -> Result<Self> {
        api.get_pokemon(name, game).await
    }

    pub async fn get_moves(&self) -> Result<MoveList<'a>> {
        let moves_futures: FuturesUnordered<_> = self
            .learn_moves
            .iter()
            .map(|mv| Move::from_name(self.api, mv.0, self.generation))
            .collect();
        let moves_results: Vec<_> = moves_futures.collect().await;

        let mut moves = HashMap::new();
        for result in moves_results {
            let move_ = result?;
            moves.insert(move_.name.clone(), move_);
        }

        Ok(MoveList::new(moves))
    }

    pub async fn get_defense_chart(&self) -> Result<TypeChart> {
        let primary_type = Type::from_name(self.api, &self.primary_type, self.generation).await?;

        if let Some(secondary_type) = &self.secondary_type {
            let secondary_type = Type::from_name(self.api, secondary_type, self.generation).await?;

            Ok(primary_type
                .defense_chart
                .combine(&secondary_type.defense_chart))
        } else {
            Ok(primary_type.defense_chart)
        }
    }

    pub async fn get_evolution_steps(&self) -> Result<EvolutionStep> {
        self.api.get_evolution_steps(&self.species).await
    }
}

pub struct Type<'a> {
    pub name: String,
    pub offense_chart: TypeChart,
    pub defense_chart: TypeChart,
    pub generation: u8,
    pub api: &'a ApiWrapper,
}

impl<'a> Type<'a> {
    pub async fn from_name(api: &'a ApiWrapper, name: &str, generation: u8) -> Result<Self> {
        api.get_type(name, generation).await
    }
}

#[derive(Default)]
pub struct Stats {
    pub hp: i64,
    pub attack: i64,
    pub defense: i64,
    pub special_attack: i64,
    pub special_defense: i64,
    pub speed: i64,
}

#[derive(Debug)]
pub struct TypeChart {
    value: HashMap<String, f32>,
}

impl Default for TypeChart {
    fn default() -> TypeChart {
        let mut chart = HashMap::new();
        let types = vec![
            "normal", "fighting", "fire", "fighting", "water", "flying", "grass", "poison",
            "electric", "ground", "psychic", "rock", "ice", "bug", "dragon", "ghost", "dark",
            "steel", "fairy",
        ];

        for type_ in types {
            chart.insert(type_.to_string(), 1.0f32);
        }

        TypeChart { value: chart }
    }
}

impl TypeChart {
    pub fn new(hashmap: HashMap<String, f32>) -> TypeChart {
        let chart = TypeChart::default();
        chart.combine(&TypeChart { value: hashmap })
    }

    pub fn get_value(&self) -> &HashMap<String, f32> {
        &self.value
    }

    pub fn get_multiplier(&self, type_: &str) -> f32 {
        *self.value.get(type_).unwrap()
    }

    fn combine(&self, chart: &TypeChart) -> TypeChart {
        let mut new_chart = HashMap::new();

        for (type_, multiplier) in &self.value {
            new_chart.insert(type_.clone(), *multiplier);
        }

        for (type_, multiplier) in &chart.value {
            if let Some(new_multiplier) = new_chart.get(type_) {
                new_chart.insert(type_.clone(), multiplier * new_multiplier);
            } else {
                new_chart.insert(type_.clone(), *multiplier);
            }
        }

        TypeChart { value: new_chart }
    }
}

pub struct Move<'a> {
    pub name: String,
    pub accuracy: Option<i64>,
    pub power: Option<i64>,
    pub pp: Option<i64>,
    pub damage_class: String,
    pub type_: String,
    pub effect: String,
    pub short_effect: String,
    pub effect_chance: Option<i64>,
    pub generation: u8,
    pub api: &'a ApiWrapper,
}

impl<'a> Move<'a> {
    pub async fn from_name(api: &'a ApiWrapper, name: &str, generation: u8) -> Result<Self> {
        api.get_move(name, generation).await
    }
}

pub struct MoveList<'a> {
    value: HashMap<String, Move<'a>>,
}

impl<'a> MoveList<'a> {
    pub fn new(hashmap: HashMap<String, Move<'a>>) -> MoveList<'a> {
        MoveList { value: hashmap }
    }

    pub fn get_map(&self) -> &HashMap<String, Move<'a>> {
        &self.value
    }
}

pub struct Ability<'a> {
    pub name: String,
    pub effect: String,
    pub short_effect: String,
    pub generation: u8,
    pub api: &'a ApiWrapper,
}

impl<'a> Ability<'a> {
    pub async fn from_name(api: &'a ApiWrapper, name: &str, generation: u8) -> Result<Self> {
        api.get_ability(name, generation).await
    }
}

#[derive(Debug)]
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

#[derive(Debug)]
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

pub fn is_stab(type_: &str, pokemon: &PokemonData) -> bool {
    if let Some(secondary_type) = &pokemon.secondary_type {
        type_ == pokemon.primary_type || type_ == secondary_type
    } else {
        type_ == pokemon.primary_type
    }
}

pub trait ResourceName: Sized {
    fn get_matches(value: &str, resource: &[String]) -> Vec<String> {
        resource
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

    fn validate(value: &str, resource: &[String]) -> ResourceResult {
        let matches = Self::get_matches(value, resource);
        if matches.iter().any(|m| *m == value) {
            ResourceResult::Valid
        } else {
            ResourceResult::Invalid(matches)
        }
    }

    fn invalid_message(value: &str, matches: &[String]) -> String {
        let resource_name = Self::resource_name();
        let mut message = format!("{resource_name} '{value}' not found.");

        if matches.len() > 20 {
            message += " Potential matches found; too many to display.";
        } else if !matches.is_empty() {
            message += &format!(" Potential matches: {}.", matches.join(" "));
        }

        message
    }

    fn try_new(value: &str, resource: &[String]) -> Result<Self> {
        match Self::validate(value, resource) {
            ResourceResult::Valid => Ok(Self::from(value.to_string())),
            ResourceResult::Invalid(matches) => bail!(Self::invalid_message(value, &matches)),
        }
    }

    fn resource_name() -> &'static str;
    fn from(value: String) -> Self;
    fn get(&self) -> &str;
}

pub enum ResourceResult {
    Valid,
    Invalid(Vec<String>),
}

pub struct PokemonName(String);
impl ResourceName for PokemonName {
    fn from(value: String) -> Self {
        Self(value)
    }

    fn get(&self) -> &str {
        &self.0
    }

    fn resource_name() -> &'static str {
        "Pokémon"
    }
}

pub struct GameName(String);
impl ResourceName for GameName {
    fn from(value: String) -> Self {
        Self(value)
    }

    fn get(&self) -> &str {
        &self.0
    }

    fn resource_name() -> &'static str {
        "Game"
    }
}

pub struct TypeName(String);
impl ResourceName for TypeName {
    fn from(value: String) -> Self {
        Self(value)
    }

    fn get(&self) -> &str {
        &self.0
    }

    fn resource_name() -> &'static str {
        "Type"
    }
}

pub struct MoveName(String);
impl ResourceName for MoveName {
    fn from(value: String) -> Self {
        Self(value)
    }

    fn get(&self) -> &str {
        &self.0
    }

    fn resource_name() -> &'static str {
        "Move"
    }
}

pub struct AbilityName(String);
impl ResourceName for AbilityName {
    fn from(value: String) -> Self {
        Self(value)
    }

    fn get(&self) -> &str {
        &self.0
    }

    fn resource_name() -> &'static str {
        "Ability"
    }
}
