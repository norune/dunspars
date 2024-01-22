use std::io::{stdout, Write};

use anyhow::Result;
use owo_colors::OwoColorize;

use crate::pokemon::{Move, MoveList, Pokemon, TypeChart};

pub struct TypeChartDisplay<'a> {
    type_chart: &'a TypeChart,
}

impl TypeChartDisplay<'_> {
    pub fn new<'a>(type_chart: &'a TypeChart) -> TypeChartDisplay<'a> {
        TypeChartDisplay { type_chart }
    }

    pub fn print_by_weakness(&self) -> Result<()> {
        let weakness_groups =
            WeaknessDisplay::from_iter(self.type_chart.get_value(), |t| (t.0.clone(), t.1.clone()));
        weakness_groups.print(" ", true)
    }
}

pub struct WeaknessDisplay {
    pub quad: Vec<String>,
    pub double: Vec<String>,
    pub neutral: Vec<String>,
    pub half: Vec<String>,
    pub quarter: Vec<String>,
    pub zero: Vec<String>,
    pub other: Vec<String>,
}

impl WeaknessDisplay {
    pub fn from_iter<C, F, T>(collection: C, mut cb: F) -> Self
    where
        C: IntoIterator<Item = T>,
        F: FnMut(T) -> (String, f32),
    {
        let mut groups = WeaknessDisplay {
            quad: vec![],
            double: vec![],
            neutral: vec![],
            half: vec![],
            quarter: vec![],
            zero: vec![],
            other: vec![],
        };

        for item in collection {
            let (str, multiplier) = cb(item);
            match multiplier {
                x if x == 4.0 => groups.quad.push(str.clone()),
                x if x == 2.0 => groups.double.push(str.clone()),
                x if x == 1.0 => groups.neutral.push(str.clone()),
                x if x == 0.5 => groups.half.push(str.clone()),
                x if x == 0.25 => groups.quarter.push(str.clone()),
                x if x == 0.0 => groups.zero.push(str.clone()),
                _ => groups.other.push(str.clone()),
            }
        }

        groups
    }

    pub fn print(&self, separator: &str, default_colors: bool) -> Result<()> {
        let mut f = stdout().lock();

        if self.quad.len() > 0 {
            let mut quad = self.quad.join(separator);
            if default_colors {
                quad = quad.red().to_string();
            }
            writeln!(f, "quad:\n{}\n", quad)?;
        }
        if self.double.len() > 0 {
            let mut double = self.double.join(separator);
            if default_colors {
                double = double.yellow().to_string();
            }
            writeln!(f, "double:\n{}\n", double)?;
        }
        if self.neutral.len() > 0 {
            let mut neutral = self.neutral.join(separator);
            if default_colors {
                neutral = neutral.white().to_string();
            }
            writeln!(f, "neutral:\n{}\n", neutral)?;
        }
        if self.half.len() > 0 {
            let mut half = self.neutral.join(separator);
            if default_colors {
                half = half.blue().to_string();
            }
            writeln!(f, "half:\n{}\n", half)?;
        }
        if self.quarter.len() > 0 {
            let mut quarter = self.quarter.join(separator);
            if default_colors {
                quarter = quarter.bright_cyan().to_string();
            }
            writeln!(f, "quarter:\n{}\n", quarter)?;
        }
        if self.zero.len() > 0 {
            let mut zero = self.zero.join(separator);
            if default_colors {
                zero = zero.purple().to_string();
            }
            writeln!(f, "zero:\n{}\n", zero)?;
        }

        Ok(())
    }
}

pub struct MoveListDisplay<'a, 'b, 'c> {
    move_list: &'a MoveList<'c>,
    pokemon: &'b Pokemon<'c>,
}

impl MoveListDisplay<'_, '_, '_> {
    pub fn new<'a, 'b, 'c>(
        move_list: &'a MoveList<'c>,
        pokemon: &'b Pokemon<'c>,
    ) -> MoveListDisplay<'a, 'b, 'c> {
        MoveListDisplay { move_list, pokemon }
    }
    pub fn print_list(&self) -> Result<()> {
        let mut f = stdout().lock();
        for move_ in self.move_list.get_value() {
            let Move {
                name,
                accuracy,
                power,
                pp,
                damage_class,
                type_,
                ..
            } = move_.1;

            let prop = format!("{name:16} ({type_} {damage_class})");
            let stats = format!(
                "power: {:3}  accuracy: {:3}  pp: {:2}",
                power.unwrap_or(0).red(),
                accuracy.unwrap_or(0).green(),
                pp.unwrap_or(0).blue()
            );

            let default_learn = ("unknown".to_string(), 0i64);
            let (learn_method, learn_level) =
                self.pokemon.moves.get(move_.0).unwrap_or(&default_learn);
            let learn = format!("{} {}", learn_method, learn_level);

            writeln!(f, "{prop:40}{stats:68}{learn}")?;
        }

        Ok(())
    }
}
