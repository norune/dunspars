#![allow(unused_imports)]

use super::{Colors, DisplayComponent};
use crate::api::ApiWrapper;
use crate::pokemon::{Pokemon, Type, TypeChart, TYPES};

use std::collections::{hash_map::Entry, HashMap};
use std::fmt;

use anyhow::Result;
use indoc::writedoc;

pub struct CoverageComponent<'a, 'b> {
    pokemon: &'a Vec<Pokemon<'b>>,
    resource: HashMap<String, Type<'b>>,
}
impl<'a, 'b> CoverageComponent<'a, 'b> {
    pub async fn try_new(pokemon: &'a Vec<Pokemon<'b>>) -> Result<Self> {
        let mut resource = HashMap::new();
        for mon in pokemon {
            let type_ = &mon.data.primary_type;
            if resource.get(type_).is_none() {
                let type_data = Type::from_name(mon.data.api, type_, mon.data.generation).await?;
                resource.insert(type_data.name.clone(), type_data);
            }

            if let Some(type_) = mon.data.secondary_type.as_ref() {
                if resource.get(type_).is_none() {
                    let type_data =
                        Type::from_name(mon.data.api, type_, mon.data.generation).await?;
                    resource.insert(type_data.name.clone(), type_data);
                }
            }
        }

        Ok(Self { pokemon, resource })
    }
}

impl fmt::Display for DisplayComponent<CoverageComponent<'_, '_>> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let CoverageComponent {
            pokemon,
            ref resource,
        } = self.context;

        let mut attack_coverage: HashMap<String, Vec<String>> = HashMap::new();
        let mut defense_coverage: HashMap<String, Vec<String>> = HashMap::new();

        for type_ in TYPES {
            attack_coverage.insert(String::from(type_), vec![]);
            defense_coverage.insert(String::from(type_), vec![]);
        }

        for Pokemon {
            data,
            defense_chart,
            ..
        } in pokemon
        {
            let pokemon_name = &data.name;

            let Type { offense_chart, .. } = resource.get(&data.primary_type).unwrap();
            self.insert_coverage(pokemon_name, offense_chart, &mut attack_coverage, true);

            if let Some(secondary_type) = data.secondary_type.as_ref() {
                let Type { offense_chart, .. } = resource.get(secondary_type).unwrap();
                self.insert_coverage(pokemon_name, offense_chart, &mut attack_coverage, true);
            }

            self.insert_coverage(pokemon_name, defense_chart, &mut defense_coverage, false);
        }

        write!(f, "{attack_coverage:#?}\n{defense_coverage:#?}")
    }
}

impl DisplayComponent<CoverageComponent<'_, '_>> {
    fn insert_coverage(
        &self,
        pokemon_name: &str,
        type_chart: &TypeChart,
        coverage: &mut HashMap<String, Vec<String>>,
        attack: bool,
    ) {
        for (type_, multiplier) in type_chart.get_value() {
            if (attack && *multiplier > 1.0) || (!attack && *multiplier < 1.0) {
                let entry = coverage.entry(type_.clone());

                if let Entry::Occupied(mut e) = entry {
                    e.get_mut().push(pokemon_name.to_string());
                }
            }
        }
    }
}
