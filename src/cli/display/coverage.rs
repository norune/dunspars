use super::{Colors, DisplayComponent};
use crate::api::ApiWrapper;
use crate::pokemon::{Pokemon, PokemonData, Type, TypeChart, TYPES};

use std::collections::{hash_map::Entry, HashMap};
use std::fmt;

use anyhow::Result;

pub struct CoverageComponent<'a, 'b> {
    pokemon: &'a Vec<Pokemon<'b>>,
    resource: HashMap<String, Type<'b>>,
}
impl<'a, 'b> CoverageComponent<'a, 'b> {
    pub async fn try_new(pokemon: &'a Vec<Pokemon<'b>>) -> Result<Self> {
        let mut resource = HashMap::new();
        for mon in pokemon {
            let PokemonData {
                api,
                generation,
                ref primary_type,
                ref secondary_type,
                ..
            } = mon.data;

            Self::add_type_to_resource(primary_type, api, generation, &mut resource).await?;

            if let Some(secondary_type) = secondary_type {
                Self::add_type_to_resource(secondary_type, api, generation, &mut resource).await?;
            }
        }

        Ok(Self { pokemon, resource })
    }

    async fn add_type_to_resource<'c>(
        type_: &str,
        api: &'b ApiWrapper,
        generation: u8,
        resource: &'c mut HashMap<String, Type<'b>>,
    ) -> Result<()> {
        if resource.get(type_).is_none() {
            let type_data = Type::from_name(api, type_, generation).await?;
            resource.insert(type_data.name.clone(), type_data);
        }
        Ok(())
    }
}

impl fmt::Display for DisplayComponent<CoverageComponent<'_, '_>> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let (offense_coverage, defense_coverage) = self.build_coverages();
        let header = self.ansi_bold(Colors::Header);

        writeln!(f, "{header}offense coverage{header:#}")?;
        self.write_coverage(f, offense_coverage)?;

        writeln!(f, "\n{header}defense coverage{header:#}")?;
        self.write_coverage(f, defense_coverage)?;

        Ok(())
    }
}

impl DisplayComponent<CoverageComponent<'_, '_>> {
    fn write_coverage(
        &self,
        f: &mut fmt::Formatter,
        mut coverage: HashMap<String, Vec<String>>,
    ) -> fmt::Result {
        let mut types = coverage
            .iter()
            .map(|t| t.0.clone())
            .collect::<Vec<String>>();
        types.sort();

        for type_ in types {
            let pokemon = coverage.get_mut(&type_).unwrap();
            let type_label;
            let covered_by;

            if pokemon.is_empty() {
                type_label = format!("{red}{type_}{red:#}", red = self.ansi_bold(Colors::Red));
                covered_by = String::from("");
            } else {
                pokemon.sort();
                type_label = format!(
                    "{green}{type_}{green:#}: ",
                    green = self.ansi(Colors::Green)
                );
                covered_by = pokemon.join(" ");
            };

            writeln!(f, "{type_label}{covered_by}")?
        }

        Ok(())
    }

    fn build_coverages(&self) -> (HashMap<String, Vec<String>>, HashMap<String, Vec<String>>) {
        let mut offense_coverage: HashMap<String, Vec<String>> = HashMap::new();
        let mut defense_coverage: HashMap<String, Vec<String>> = HashMap::new();

        let CoverageComponent {
            pokemon,
            ref resource,
        } = self.context;

        for type_ in TYPES {
            offense_coverage.insert(String::from(type_), vec![]);
            defense_coverage.insert(String::from(type_), vec![]);
        }

        for Pokemon {
            data,
            defense_chart,
            ..
        } in pokemon
        {
            let pokemon_name = &data.name;

            let Type {
                name: type_name,
                offense_chart,
                ..
            } = resource.get(&data.primary_type).unwrap();
            self.add_pokemon_to_coverage(
                pokemon_name,
                offense_chart,
                &mut offense_coverage,
                true,
                type_name,
            );

            if let Some(secondary_type) = data.secondary_type.as_ref() {
                let Type {
                    name: type_name,
                    offense_chart,
                    ..
                } = resource.get(secondary_type).unwrap();
                self.add_pokemon_to_coverage(
                    pokemon_name,
                    offense_chart,
                    &mut offense_coverage,
                    true,
                    type_name,
                );
            }

            self.add_pokemon_to_coverage(
                pokemon_name,
                defense_chart,
                &mut defense_coverage,
                false,
                "",
            );
        }

        (offense_coverage, defense_coverage)
    }

    fn add_pokemon_to_coverage(
        &self,
        pokemon_name: &str,
        type_chart: &TypeChart,
        coverage: &mut HashMap<String, Vec<String>>,
        attack: bool,
        tag: &str,
    ) {
        for (type_, multiplier) in type_chart.get_value() {
            if (attack && *multiplier > 1.0) || (!attack && *multiplier < 1.0) {
                let entry = coverage.entry(type_.clone());

                if let Entry::Occupied(mut entry) = entry {
                    let tag = if tag.is_empty() {
                        multiplier.to_string()
                    } else {
                        String::from(tag)
                    };

                    let pokemon = format!(
                        "{green}{pokemon_name}{green:#} ({tag})",
                        green = self.ansi(Colors::Cyan)
                    );
                    entry.get_mut().push(pokemon);
                }
            }
        }
    }
}
