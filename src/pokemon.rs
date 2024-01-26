use std::collections::HashMap;

use anyhow::Result;
use futures::future::join_all;

use crate::api::ApiWrapper;

pub struct Pokemon<'a> {
    pub name: String,
    pub primary_type: String,
    pub secondary_type: Option<String>,
    pub moves: HashMap<String, (String, i64)>,
    pub version: String,
    pub generation: u8,
    pub stats: Stats,
    pub api: &'a ApiWrapper,
}

impl<'a> Pokemon<'a> {
    pub async fn from_name(api: &'a ApiWrapper, name: &str, version: &str) -> Result<Self> {
        api.get_pokemon(name, version).await
    }

    pub async fn get_moves(&self) -> Result<MoveList> {
        let moves_futures = self
            .moves
            .iter()
            .map(|mv| Move::from_name(self.api, mv.0))
            .collect::<Vec<_>>();
        let moves_results = join_all(moves_futures).await;

        let mut moves = HashMap::new();
        for result in moves_results {
            let move_ = result?;
            moves.insert(move_.name.clone(), move_);
        }

        Ok(MoveList::from_hashmap(moves))
    }

    pub async fn get_defense_chart(&self) -> Result<TypeChart> {
        let primary_type = Type::from_name(self.api, &self.primary_type).await?;

        if let Some(secondary_type) = &self.secondary_type {
            let secondary_type = Type::from_name(self.api, secondary_type).await?;

            Ok(primary_type
                .defense_chart
                .combine(&secondary_type.defense_chart))
        } else {
            Ok(primary_type.defense_chart)
        }
    }
}

pub struct Type<'a> {
    pub name: String,
    pub offense_chart: TypeChart,
    pub defense_chart: TypeChart,
    pub api: &'a ApiWrapper,
}

impl<'a> Type<'a> {
    pub async fn from_name(api: &'a ApiWrapper, name: &str) -> Result<Self> {
        api.get_type(name).await
    }
}

pub struct Stats {
    pub hp: i64,
    pub attack: i64,
    pub defense: i64,
    pub special_attack: i64,
    pub special_defense: i64,
    pub speed: i64,
}

impl Default for Stats {
    fn default() -> Self {
        Self {
            hp: 0,
            attack: 0,
            defense: 0,
            special_attack: 0,
            special_defense: 0,
            speed: 0,
        }
    }
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
    pub fn from_hashmap(hashmap: HashMap<String, f32>) -> TypeChart {
        let chart = TypeChart::default();
        chart.combine(&TypeChart { value: hashmap })
    }

    pub fn get_value(&self) -> &HashMap<String, f32> {
        &self.value
    }

    pub fn get_multiplier(&self, type_: &str) -> f32 {
        self.value.get(type_).unwrap().clone()
    }

    fn combine(&self, chart: &TypeChart) -> TypeChart {
        let mut new_chart = HashMap::new();

        for (type_, multiplier) in &self.value {
            new_chart.insert(type_.clone(), multiplier.clone());
        }

        for (type_, multiplier) in &chart.value {
            if let Some(new_multiplier) = new_chart.get(type_) {
                new_chart.insert(type_.clone(), multiplier * new_multiplier);
            } else {
                new_chart.insert(type_.clone(), multiplier.clone());
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
    pub api: &'a ApiWrapper,
    pub effect: String,
    pub effect_short: String,
}

impl<'a> Move<'a> {
    pub async fn from_name(api: &'a ApiWrapper, name: &str) -> Result<Self> {
        api.get_move(&name).await
    }
}

pub struct MoveList<'a> {
    value: HashMap<String, Move<'a>>,
}

impl MoveList<'_> {
    pub fn from_hashmap<'a>(hashmap: HashMap<String, Move<'a>>) -> MoveList<'a> {
        MoveList { value: hashmap }
    }

    pub fn get_value(&self) -> &HashMap<String, Move<'_>> {
        &self.value
    }
}

pub fn is_stab(type_: &str, pokemon: &Pokemon) -> bool {
    if let Some(secondary_type) = &pokemon.secondary_type {
        type_ == pokemon.primary_type || &type_ == secondary_type
    } else {
        type_ == pokemon.primary_type
    }
}
