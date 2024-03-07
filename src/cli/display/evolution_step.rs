use super::{Colors, DisplayComponent};
use crate::models::{EvolutionMethod, EvolutionStep};

use std::fmt;

impl fmt::Display for DisplayComponent<&EvolutionStep> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(
            f,
            "{header}evolution{header:#}",
            header = self.ansi_bold(Colors::Header)
        )?;
        self.traverse_dfs(f, self.context, 0)?;
        Ok(())
    }
}

impl DisplayComponent<&EvolutionStep> {
    pub fn traverse_dfs(
        &self,
        f: &mut fmt::Formatter,
        node: &EvolutionStep,
        depth: usize,
    ) -> fmt::Result {
        self.write_step(f, node, depth)?;
        for child in &node.evolves_to {
            writeln!(f)?;
            self.traverse_dfs(f, child, depth + 1)?;
        }

        Ok(())
    }

    fn write_step(
        &self,
        f: &mut fmt::Formatter,
        step: &EvolutionStep,
        depth: usize,
    ) -> fmt::Result {
        let methods = self.format_methods(&step.methods);
        write!(
            f,
            "{indentation}{green}{species}{green:#} {methods}",
            indentation = "  ".repeat(depth),
            green = self.ansi(Colors::Green),
            species = step.name
        )
    }

    fn format_methods(&self, methods: &[EvolutionMethod]) -> String {
        methods
            .iter()
            .map(|m| self.format_method(m))
            .collect::<Vec<String>>()
            .join(" / ")
    }

    fn format_method(&self, method: &EvolutionMethod) -> String {
        let EvolutionMethod {
            trigger,
            item,
            gender,
            held_item,
            known_move,
            known_move_type,
            location,
            min_level,
            min_happiness,
            min_beauty,
            min_affection,
            needs_overworld_rain,
            party_species,
            party_type,
            relative_physical_stats,
            time_of_day,
            trade_species,
            turn_upside_down,
        } = method;
        let mut output = format!("{blue}{trigger}{blue:#}", blue = self.ansi(Colors::Blue));

        if let Some(item) = item {
            output += &format!(" {item}");
        }

        if let Some(gender_int) = gender {
            let gender = match gender_int {
                1 => "female",
                2 => "male",
                _ => "other",
            };
            output += &format!(" gender-{gender}");
        }

        if let Some(held_item) = held_item {
            output += &format!(" {held_item}");
        }

        if let Some(known_move) = known_move {
            output += &format!(" {known_move}");
        }

        if let Some(known_move_type) = known_move_type {
            output += &format!(" {known_move_type}");
        }

        if let Some(location) = location {
            output += &format!(" {location}");
        }

        if let Some(min_level) = min_level {
            output += &format!(" level-{min_level}");
        }

        if let Some(min_happiness) = min_happiness {
            output += &format!(" happiness-{min_happiness}");
        }

        if let Some(min_beauty) = min_beauty {
            output += &format!(" beauty-{min_beauty}");
        }

        if let Some(min_affection) = min_affection {
            output += &format!(" affection-{min_affection}");
        }

        if let Some(needs_overworld_rain) = needs_overworld_rain {
            if *needs_overworld_rain {
                output += " rain";
            }
        }

        if let Some(party_species) = party_species {
            output += &format!(" {party_species}");
        }

        if let Some(party_type) = party_type {
            output += &format!(" {party_type}");
        }

        if let Some(relative_physical_stats) = relative_physical_stats {
            output += &format!(" physical-{relative_physical_stats}");
        }

        if let Some(time_of_day) = time_of_day {
            output += &format!(" {time_of_day}");
        }

        if let Some(trade_species) = trade_species {
            output += &format!(" {trade_species}");
        }

        if let Some(turn_upside_down) = turn_upside_down {
            if *turn_upside_down {
                output += " upside-down";
            }
        }

        output
    }
}
