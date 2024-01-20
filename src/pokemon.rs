use std::collections::HashMap;
use std::{fmt, vec};

use anyhow::Result;
use futures::future::join_all;
use owo_colors::OwoColorize;

use crate::api::ApiWrapper;

pub struct Pokemon<'a> {
    pub name: String,
    pub primary_type: String,
    pub secondary_type: Option<String>,
    pub moves: Vec<String>,
    pub api: &'a ApiWrapper,
}

impl<'a> Pokemon<'a> {
    pub async fn from_name(api: &'a ApiWrapper, name: &str, version: &str) -> Result<Self> {
        let pokemon = api.get_pokemon(name, version).await?;
        Ok(pokemon)
    }

    pub async fn get_moves(&self) -> Result<Vec<Move>> {
        let moves_futures = self
            .moves
            .iter()
            .map(|mv| self.api.get_move(mv))
            .collect::<Vec<_>>();
        let moves_results = join_all(moves_futures).await;

        let mut moves = vec![];
        for mv in moves_results {
            moves.push(mv?);
        }

        Ok(moves)
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
        let type_ = api.get_type(name).await?;
        Ok(type_)
    }
}

#[derive(Debug)]
pub struct TypeChart(HashMap<String, f32>);

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

        TypeChart(chart)
    }
}

impl TypeChart {
    pub fn from_hashmap(hashmap: HashMap<String, f32>) -> TypeChart {
        let chart = TypeChart::default();
        chart.combine(&TypeChart(hashmap))
    }

    fn combine(&self, chart: &TypeChart) -> TypeChart {
        let mut new_chart = HashMap::new();

        for (type_, multiplier) in &self.0 {
            new_chart.insert(type_.clone(), multiplier.clone());
        }

        for (type_, multiplier) in &chart.0 {
            if let Some(new_multiplier) = new_chart.get(type_) {
                new_chart.insert(type_.clone(), multiplier * new_multiplier);
            } else {
                new_chart.insert(type_.clone(), multiplier.clone());
            }
        }

        TypeChart(new_chart)
    }

    pub fn group_by_multiplier(&self) -> TypeChartGrouped {
        let mut quad = vec![];
        let mut double = vec![];
        let mut neutral = vec![];
        let mut half = vec![];
        let mut zero = vec![];
        let mut other = vec![];

        for (type_, multiplier) in &self.0 {
            match multiplier {
                x if *x == 4.0 => quad.push(type_.clone()),
                x if *x == 2.0 => double.push(type_.clone()),
                x if *x == 1.0 => neutral.push(type_.clone()),
                x if *x == 0.5 => half.push(type_.clone()),
                x if *x == 0.0 => zero.push(type_.clone()),
                _ => other.push(type_.clone()),
            }
        }

        TypeChartGrouped {
            quad,
            double,
            neutral,
            half,
            zero,
            other,
        }
    }
}

pub struct TypeChartGrouped {
    pub quad: Vec<String>,
    pub double: Vec<String>,
    pub neutral: Vec<String>,
    pub half: Vec<String>,
    pub zero: Vec<String>,
    #[allow(dead_code)]
    pub other: Vec<String>,
}

impl fmt::Display for TypeChartGrouped {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.quad.len() > 0 {
            writeln!(f, "quad: {}", self.quad.join(" ").red())?;
        }
        if self.double.len() > 0 {
            writeln!(f, "double: {}", self.double.join(" ").bright_yellow())?;
        }
        if self.neutral.len() > 0 {
            writeln!(f, "neutral: {}", self.neutral.join(" "))?;
        }
        if self.half.len() > 0 {
            writeln!(f, "half: {}", self.half.join(" ").bright_blue())?;
        }
        if self.zero.len() > 0 {
            writeln!(f, "zero: {}", self.zero.join(" ").bright_purple())?;
        }

        Ok(())
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
}

impl<'a> fmt::Display for Move<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let Move {
            name,
            accuracy,
            power,
            pp,
            damage_class,
            type_,
            ..
        } = self;

        let left = format!("{name} ({type_} {damage_class})");
        let right = format!(
            "power: {:3}  accuracy: {:3}  pp: {:2}",
            power.unwrap_or(0).red(),
            accuracy.unwrap_or(0).green(),
            pp.unwrap_or(0).blue()
        );

        write!(f, "{left:40}{right}")?;

        Ok(())
    }
}
