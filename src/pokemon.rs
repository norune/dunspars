use std::error::Error;
use std::collections::HashMap;
use std::{fmt, vec};

use crate::api::ApiWrapper;

pub struct Pokemon<'a> {
    pub name: String,
    pub primary_type: String,
    pub secondary_type: Option<String>,
    api: &'a ApiWrapper,
}

impl<'a> Pokemon<'a> {
    pub async fn from_name(api: &'a ApiWrapper, name: &str) -> Result<Self, Box<dyn Error>> {
        let pokemon = api.get_pokemon(name).await?;

        Ok(Pokemon {
            name: pokemon.name,
            primary_type: pokemon.primary_type,
            secondary_type: pokemon.secondary_type,
            api
        })
    }

    pub async fn get_defense_chart(&self) -> Result<TypeChart, Box<dyn Error>> {
        let primary_type = self.api.get_type(&self.primary_type).await?;
        let primary_chart = TypeChart(primary_type.defensive_chart);

        if let Some(type_) = &self.secondary_type {
            let secondary_type = self.api.get_type(&type_).await?;
            let secondary_chart = TypeChart(secondary_type.defensive_chart);

            Ok(primary_chart.combine(&secondary_chart))
        } else {
            Ok(primary_chart)
        }
    }
}

#[derive(Debug)]
pub struct TypeChart(HashMap<String, f32>);

impl TypeChart {
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
}

impl fmt::Display for TypeChart {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut zero = vec![];
        let mut half = vec![];
        let mut double = vec![];
        let mut quad = vec![];

        for (type_, multiplier) in &self.0 {
            match multiplier {
                x if *x == 0.0 => zero.push(type_.clone()),
                x if *x == 0.5 => half.push(type_.clone()),
                x if *x == 2.0 => double.push(type_.clone()),
                x if *x == 4.0 => quad.push(type_.clone()),
                _ => ()
            }
        }
        
        write!(f, "quad: {0}\ndouble: {1}\nhalf: {2}\nzero: {3}\n",
            quad.join(" "), double.join(" "), half.join(" "), zero.join(" "))
    }
}