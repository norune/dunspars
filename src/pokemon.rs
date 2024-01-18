use std::collections::HashMap;
use std::{fmt, vec};

use anyhow::Result;
use owo_colors::OwoColorize;

use crate::api::ApiWrapper;

pub struct Pokemon<'a> {
    pub name: String,
    pub primary_type: String,
    pub secondary_type: Option<String>,
    api: &'a ApiWrapper,
}

impl<'a> Pokemon<'a> {
    pub async fn from_name(api: &'a ApiWrapper, name: &str) -> Result<Self> {
        let pokemon = api.get_pokemon(name).await?;

        Ok(Pokemon {
            name: pokemon.name,
            primary_type: pokemon.primary_type,
            secondary_type: pokemon.secondary_type,
            api,
        })
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
    #[allow(dead_code)]
    api: &'a ApiWrapper,
}

impl<'a> Type<'a> {
    pub async fn from_name(api: &'a ApiWrapper, name: &str) -> Result<Self> {
        let type_ = api.get_type(name).await?;
        let offense_chart = TypeChart::from_hashmap(type_.offense_chart);
        let defense_chart = TypeChart::from_hashmap(type_.defense_chart);

        Ok(Type {
            name: type_.name,
            offense_chart,
            defense_chart,
            api,
        })
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
