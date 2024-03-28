use super::{Colors, DisplayComponent};
use crate::cli::utils::is_stab;
use crate::models::{FromDb, Move, Pokemon, Type, TypeChart, TypeCharts, TYPES};

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

        for pokemon in pokemon {
            let move_list = pokemon.get_move_list(db).unwrap();

            // If the pokemon's move list is empty (i.e. non-custom), use its types as its offensive coverage
            if move_list.is_empty() {
                let primary_type =
                    Type::from_db(&pokemon.primary_type, pokemon.generation, db).unwrap();
                self.add_type_coverage(pokemon, &primary_type.offense_chart, &mut offense_coverage);

                if let Some(secondary_type) = pokemon.secondary_type.as_ref() {
                    let secondary_type =
                        Type::from_db(secondary_type, pokemon.generation, db).unwrap();
                    self.add_type_coverage(
                        pokemon,
                        &secondary_type.offense_chart,
                        &mut offense_coverage,
                    );
                }
            } else {
                for move_ in move_list.get_list().values() {
                    if move_.is_combat() {
                        self.add_move_coverage(pokemon, move_, &mut offense_coverage);
                    }
                }
            }

            let defense_chart = pokemon.get_defense_chart(db).unwrap();
            self.add_type_coverage(pokemon, &defense_chart, &mut defense_coverage);
        }

        (offense_coverage, defense_coverage)
    }

    fn add_move_coverage(
        &self,
        pokemon: &Pokemon,
        move_: &Move,
        coverage: &mut HashMap<String, Vec<String>>,
    ) {
        let move_type = Type::from_db(&move_.type_, move_.generation, self.context.db).unwrap();
        let covered_types = self.get_covered_types(&move_type.offense_chart);
        for type_ in covered_types {
            let mut tag = move_.name.clone();
            if is_stab(&move_.type_, pokemon) {
                tag += "+";
            }
            self.add_to_coverage(&pokemon.name, &tag, &type_, coverage);
        }
    }

    fn add_type_coverage(
        &self,
        pokemon: &Pokemon,
        type_chart: &impl TypeChart,
        coverage: &mut HashMap<String, Vec<String>>,
    ) {
        let covered_types = self.get_covered_types(type_chart);
        for type_ in covered_types {
            let tag = match type_chart.get_type() {
                TypeCharts::Offense => type_chart.get_label(),
                TypeCharts::Defense => {
                    let multiplier = type_chart.get_multiplier(&type_);
                    multiplier.to_string()
                }
            };
            self.add_to_coverage(&pokemon.name, &tag, &type_, coverage);
        }
    }

    fn get_covered_types(&self, type_chart: &impl TypeChart) -> Vec<String> {
        type_chart
            .get_chart()
            .iter()
            .filter_map(|(type_, multiplier)| {
                let covered = match type_chart.get_type() {
                    TypeCharts::Offense => *multiplier > 1.0,
                    TypeCharts::Defense => *multiplier < 1.0,
                };
                if covered {
                    Some(type_.clone())
                } else {
                    None
                }
            })
            .collect()
    }

    fn add_to_coverage(
        &self,
        name: &str,
        tag: &str,
        type_: &str,
        coverage: &mut HashMap<String, Vec<String>>,
    ) {
        let entry = coverage.entry(String::from(type_));

        if let Entry::Occupied(mut entry) = entry {
            let pokemon = format!(
                "{green}{name}{green:#} ({tag})",
                green = self.ansi(Colors::Cyan)
            );
            entry.get_mut().push(pokemon);
        }
    }
}
