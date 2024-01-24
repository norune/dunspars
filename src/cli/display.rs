use std::io::{stdout, Write};

use anyhow::Result;
use owo_colors::{OwoColorize, Style};

use crate::pokemon::{Move, MoveList, Pokemon, TypeChart};

pub struct TypeChartDisplay<'a> {
    type_chart: &'a TypeChart,
}

impl<'a> TypeChartDisplay<'a> {
    pub fn new(type_chart: &'a TypeChart) -> Self {
        TypeChartDisplay { type_chart }
    }

    pub fn print(&self) -> Result<()> {
        let weakness_groups = WeaknessGroups::from_iter(self.type_chart.get_value(), |t| {
            Some((t.0.clone(), t.1.clone()))
        });
        self.print_weakness_groups(weakness_groups)
    }

    fn print_weakness_groups(&self, weakness_groups: WeaknessGroups<String>) -> Result<()> {
        let mut f = stdout().lock();

        if weakness_groups.quad.len() > 0 {
            let quad = weakness_groups.quad.join(" ");
            writeln!(f, "quad:\n{}\n", quad.red())?;
        }
        if weakness_groups.double.len() > 0 {
            let double = weakness_groups.double.join(" ");
            writeln!(f, "double:\n{}\n", double.yellow())?;
        }
        if weakness_groups.neutral.len() > 0 {
            let neutral = weakness_groups.neutral.join(" ");
            writeln!(f, "neutral:\n{}\n", neutral.green())?;
        }
        if weakness_groups.half.len() > 0 {
            let half = weakness_groups.half.join(" ");
            writeln!(f, "half:\n{}\n", half.blue())?;
        }
        if weakness_groups.quarter.len() > 0 {
            let quarter = weakness_groups.quarter.join(" ");
            writeln!(f, "quarter:\n{}\n", quarter.bright_cyan())?;
        }
        if weakness_groups.zero.len() > 0 {
            let zero = weakness_groups.zero.join(" ");
            writeln!(f, "zero:\n{}\n", zero.purple())?;
        }

        Ok(())
    }
}

pub struct MoveWeakDisplay<'a, 'b, 'c, 'd> {
    type_chart: &'a TypeChart,
    move_list: &'b MoveList<'d>,
    attacker: &'c Pokemon<'d>,
}

impl<'a, 'b, 'c, 'd> MoveWeakDisplay<'a, 'b, 'c, 'd> {
    pub fn new(
        type_chart: &'a TypeChart,
        move_list: &'b MoveList<'d>,
        attacker: &'c Pokemon<'d>,
    ) -> Self {
        MoveWeakDisplay {
            type_chart,
            move_list,
            attacker,
        }
    }

    pub fn print(&self) -> Result<()> {
        let weakness_groups = WeaknessGroups::from_iter(self.move_list.get_value(), |move_| {
            if move_.1.damage_class != "status" {
                let multiplier = self.type_chart.get_multiplier(&move_.1.type_);
                Some((move_.1, multiplier))
            } else {
                None
            }
        });
        self.print_weakness_groups(weakness_groups)
    }

    fn print_weakness_groups(&self, weakness_groups: WeaknessGroups<&Move>) -> Result<()> {
        let mut f = stdout().lock();

        if weakness_groups.quad.len() > 0 {
            let style = Style::new().red();
            write!(f, "quad: ")?;
            self.print_group(weakness_groups.quad, style)?;
        }
        if weakness_groups.double.len() > 0 {
            let style = Style::new().yellow();
            write!(f, "double: ")?;
            self.print_group(weakness_groups.double, style)?;
        }
        if weakness_groups.neutral.len() > 0 {
            let style = Style::new().green();
            write!(f, "neutral: ")?;
            self.print_group(weakness_groups.neutral, style)?;
        }
        if weakness_groups.half.len() > 0 {
            let style = Style::new().blue();
            write!(f, "half: ")?;
            self.print_group(weakness_groups.half, style)?;
        }
        if weakness_groups.quarter.len() > 0 {
            let style = Style::new().bright_cyan();
            write!(f, "quarter: ")?;
            self.print_group(weakness_groups.quarter, style)?;
        }
        if weakness_groups.zero.len() > 0 {
            let style = Style::new().purple();
            write!(f, "zero: ")?;
            self.print_group(weakness_groups.zero, style)?;
        }

        Ok(())
    }

    fn print_group(&self, group: Vec<&Move>, style: Style) -> Result<()> {
        let mut f = stdout().lock();
        for move_ in group {
            let damage_class = match move_.damage_class.as_str() {
                "special" => "s",
                "physical" => "p",
                _ => "?",
            };
            let mut move_format = format!("{}({})", move_.name, damage_class);

            let is_stab = if let Some(secondary_type) = &self.attacker.secondary_type {
                move_.type_ == self.attacker.primary_type || &move_.type_ == secondary_type
            } else {
                move_.type_ == self.attacker.primary_type
            };
            if is_stab {
                move_format = move_format.white().on_bright_black().to_string();
            } else {
                move_format = move_format.style(style).to_string();
            }

            write!(f, "{} ", move_format)?;
        }
        writeln!(f, "\n")?;
        Ok(())
    }
}

pub struct WeaknessGroups<T> {
    pub quad: Vec<T>,
    pub double: Vec<T>,
    pub neutral: Vec<T>,
    pub half: Vec<T>,
    pub quarter: Vec<T>,
    pub zero: Vec<T>,
    pub other: Vec<T>,
}

impl<T> WeaknessGroups<T> {
    pub fn from_iter<C, F, I>(collection: C, mut cb: F) -> Self
    where
        C: IntoIterator<Item = I>,
        F: FnMut(I) -> Option<(T, f32)>,
    {
        let mut groups = WeaknessGroups {
            quad: vec![],
            double: vec![],
            neutral: vec![],
            half: vec![],
            quarter: vec![],
            zero: vec![],
            other: vec![],
        };

        for element in collection {
            if let Some(result) = cb(element) {
                let (item, multiplier) = result;
                match multiplier {
                    x if x == 4.0 => groups.quad.push(item),
                    x if x == 2.0 => groups.double.push(item),
                    x if x == 1.0 => groups.neutral.push(item),
                    x if x == 0.5 => groups.half.push(item),
                    x if x == 0.25 => groups.quarter.push(item),
                    x if x == 0.0 => groups.zero.push(item),
                    _ => groups.other.push(item),
                }
            }
        }

        groups
    }
}

pub struct MoveListDisplay<'a, 'b, 'c> {
    move_list: &'a MoveList<'c>,
    pokemon: &'b Pokemon<'c>,
}

impl<'a, 'b, 'c> MoveListDisplay<'a, 'b, 'c> {
    pub fn new(move_list: &'a MoveList<'c>, pokemon: &'b Pokemon<'c>) -> Self {
        MoveListDisplay { move_list, pokemon }
    }

    pub fn print(&self) -> Result<()> {
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

            let default_learn = ("".to_string(), 0i64);
            let (learn_method, level) = self.pokemon.moves.get(move_.0).unwrap_or(&default_learn);
            let learn_level = if *level == 0i64 {
                "".to_string()
            } else {
                level.to_string()
            };
            let learn = format!("{} {}", learn_method, learn_level);

            writeln!(f, "{prop:40}{stats:68}{learn}")?;
        }

        Ok(())
    }
}

pub struct MoveDisplay<'a, 'b> {
    move_: &'a Move<'b>,
}

impl<'a, 'b> MoveDisplay<'a, 'b> {
    pub fn new(move_: &'a Move<'b>) -> Self {
        MoveDisplay { move_ }
    }

    pub fn print(&self) -> Result<()> {
        let mut f = stdout().lock();
        let stats = format!(
            "power: {}  accuracy: {}  pp: {}",
            self.move_.power.unwrap_or(0).red(),
            self.move_.accuracy.unwrap_or(0).green(),
            self.move_.pp.unwrap_or(0).blue()
        );
        writeln!(
            f,
            "{} ({} {})\n{}\n{}",
            self.move_.name, self.move_.type_, self.move_.damage_class, stats, self.move_.effect
        )?;
        Ok(())
    }
}
