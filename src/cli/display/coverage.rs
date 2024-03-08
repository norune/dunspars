use super::{Colors, DisplayComponent};
use crate::models::{Pokemon, Type, TypeChart, TypeCharts, TYPES};

use std::collections::{hash_map::Entry, HashMap};
use std::fmt;

use rusqlite::Connection;

pub struct CoverageComponent<'a> {
    pub pokemon: &'a Vec<Pokemon>,
    pub db: &'a Connection,
}

impl fmt::Display for DisplayComponent<CoverageComponent<'_>> {
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

impl DisplayComponent<CoverageComponent<'_>> {
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

        let CoverageComponent { pokemon, db } = self.context;

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

            let Type { offense_chart, .. } =
                Type::from_name(&data.primary_type, data.generation, db).unwrap();
            self.add_pokemon_to_coverage(pokemon_name, &offense_chart, &mut offense_coverage);

            if let Some(secondary_type) = data.secondary_type.as_ref() {
                let Type { offense_chart, .. } =
                    Type::from_name(secondary_type, data.generation, db).unwrap();
                self.add_pokemon_to_coverage(pokemon_name, &offense_chart, &mut offense_coverage);
            }

            self.add_pokemon_to_coverage(pokemon_name, defense_chart, &mut defense_coverage);
        }

        (offense_coverage, defense_coverage)
    }

    fn add_pokemon_to_coverage(
        &self,
        pokemon_name: &str,
        type_chart: &impl TypeChart,
        coverage: &mut HashMap<String, Vec<String>>,
    ) {
        for (type_, multiplier) in type_chart.get_chart() {
            let (covered, tag) = match type_chart.get_type() {
                TypeCharts::Offense => (*multiplier > 1.0, type_chart.get_label()),
                TypeCharts::Defense => (*multiplier < 1.0, multiplier.to_string()),
            };

            if covered {
                let entry = coverage.entry(type_.clone());

                if let Entry::Occupied(mut entry) = entry {
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
