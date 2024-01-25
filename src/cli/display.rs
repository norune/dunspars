use std::io::{stdout, Write};

use anyhow::Result;
use indoc::writedoc;
use owo_colors::{colors::xterm, Style};

use crate::pokemon::{Move, MoveList, Pokemon, TypeChart};

pub struct StyleSheet {
    pub header: Style,
    pub quad: Style,
    pub double: Style,
    pub neutral: Style,
    pub half: Style,
    pub quarter: Style,
    pub zero: Style,
    pub power: Style,
    pub accuracy: Style,
    pub pp: Style,
}

impl Default for StyleSheet {
    fn default() -> Self {
        Self {
            header: Style::new().bright_green().bold(),
            quad: Style::new().red(),
            double: Style::new().yellow(),
            neutral: Style::new().green(),
            half: Style::new().blue(),
            quarter: Style::new().bright_cyan(),
            zero: Style::new().purple(),
            power: Style::new().fg::<xterm::FlushOrange>(),
            accuracy: Style::new().fg::<xterm::FernGreen>(),
            pp: Style::new().fg::<xterm::ScienceBlue>(),
        }
    }
}

pub struct TypeChartDisplay<'a> {
    type_chart: &'a TypeChart,
    css: StyleSheet,
}

impl<'a> TypeChartDisplay<'a> {
    pub fn new(type_chart: &'a TypeChart) -> Self {
        TypeChartDisplay {
            type_chart,
            css: StyleSheet::default(),
        }
    }

    pub fn print(&self) -> Result<()> {
        let weakness_groups = WeaknessGroups::new(self.type_chart.get_value(), |t| {
            Some((t.0.clone(), t.1.clone()))
        });

        println!("\n{}", self.css.header.style("defense chart"));
        self.print_weakness_groups(weakness_groups)
    }

    fn print_weakness_groups(&self, weakness_groups: WeaknessGroups<String>) -> Result<()> {
        let mut f = stdout().lock();

        if weakness_groups.quad.len() > 0 {
            let quad = weakness_groups.quad.join(" ");
            writeln!(f, "quad:\n{}\n", self.css.quad.style(quad))?;
        }
        if weakness_groups.double.len() > 0 {
            let double = weakness_groups.double.join(" ");
            writeln!(f, "double:\n{}\n", self.css.double.style(double))?;
        }
        if weakness_groups.neutral.len() > 0 {
            let neutral = weakness_groups.neutral.join(" ");
            writeln!(f, "neutral:\n{}\n", self.css.neutral.style(neutral))?;
        }
        if weakness_groups.half.len() > 0 {
            let half = weakness_groups.half.join(" ");
            writeln!(f, "half:\n{}\n", self.css.half.style(half))?;
        }
        if weakness_groups.quarter.len() > 0 {
            let quarter = weakness_groups.quarter.join(" ");
            writeln!(f, "quarter:\n{}\n", self.css.quarter.style(quarter))?;
        }
        if weakness_groups.zero.len() > 0 {
            let zero = weakness_groups.zero.join(" ");
            writeln!(f, "zero:\n{}\n", self.css.zero.style(zero))?;
        }
        if weakness_groups.other.len() > 0 {
            let other = weakness_groups.other.join(" ");
            writeln!(f, "zero:\n{}\n", other)?;
        }

        Ok(())
    }
}

pub struct MatchDisplay<'a, 'b, 'c, 'd> {
    type_chart: &'a TypeChart,
    move_list: &'b MoveList<'d>,
    attacker: &'c Pokemon<'d>,
    stab_only: bool,
    css: StyleSheet,
}

impl<'a, 'b, 'c, 'd> MatchDisplay<'a, 'b, 'c, 'd> {
    pub fn new(
        type_chart: &'a TypeChart,
        move_list: &'b MoveList<'d>,
        attacker: &'c Pokemon<'d>,
        stab_only: bool,
    ) -> Self {
        MatchDisplay {
            type_chart,
            move_list,
            attacker,
            stab_only,
            css: StyleSheet::default(),
        }
    }

    pub fn print(&self) -> Result<()> {
        let weakness_groups = WeaknessGroups::new(self.move_list.get_value(), |move_| {
            let stab_qualified = !self.stab_only || self.is_stab(&move_.1.type_);
            if move_.1.damage_class != "status" && stab_qualified {
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
            write!(f, "quad: ")?;
            self.print_group(weakness_groups.quad, self.css.quad)?;
        }
        if weakness_groups.double.len() > 0 {
            write!(f, "double: ")?;
            self.print_group(weakness_groups.double, self.css.double)?;
        }
        if weakness_groups.neutral.len() > 0 {
            write!(f, "neutral: ")?;
            self.print_group(weakness_groups.neutral, self.css.neutral)?;
        }
        if weakness_groups.half.len() > 0 {
            write!(f, "half: ")?;
            self.print_group(weakness_groups.half, self.css.half)?;
        }
        if weakness_groups.quarter.len() > 0 {
            write!(f, "quarter: ")?;
            self.print_group(weakness_groups.quarter, self.css.quarter)?;
        }
        if weakness_groups.zero.len() > 0 {
            write!(f, "zero: ")?;
            self.print_group(weakness_groups.zero, self.css.zero)?;
        }
        if weakness_groups.other.len() > 0 {
            write!(f, "other: ")?;
            self.print_group(weakness_groups.other, self.css.neutral)?;
        }

        Ok(())
    }

    fn print_group(&self, group: Vec<&Move>, group_style: Style) -> Result<()> {
        let mut f = stdout().lock();
        for move_ in group {
            let damage_class = match move_.damage_class.as_str() {
                "special" => "s",
                "physical" => "p",
                _ => "?",
            };
            let move_string = format!("{}({})", move_.name, damage_class);
            let styled_move;
            if self.is_stab(&move_.type_) {
                styled_move = group_style.underline().style(move_string);
            } else {
                styled_move = group_style.style(move_string);
            }

            write!(f, "{} ", styled_move)?;
        }
        writeln!(f, "{}", "\n")?;
        Ok(())
    }

    fn is_stab(&self, type_: &str) -> bool {
        if let Some(secondary_type) = &self.attacker.secondary_type {
            type_ == self.attacker.primary_type || &type_ == secondary_type
        } else {
            type_ == self.attacker.primary_type
        }
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
    pub fn new<C, F, I>(collection: C, mut cb: F) -> Self
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
    css: StyleSheet,
}

impl<'a, 'b, 'c> MoveListDisplay<'a, 'b, 'c> {
    pub fn new(move_list: &'a MoveList<'c>, pokemon: &'b Pokemon<'c>) -> Self {
        MoveListDisplay {
            move_list,
            pokemon,
            css: StyleSheet::default(),
        }
    }

    pub fn print(&self) -> Result<()> {
        let mut f = stdout().lock();

        println!("\n{}", self.css.header.style("moves"));

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
                self.css.power.style(power.unwrap_or(0)),
                self.css.accuracy.style(accuracy.unwrap_or(0)),
                self.css.pp.style(pp.unwrap_or(0))
            );

            let default_learn = ("".to_string(), 0i64);
            let (learn_method, level) = self.pokemon.moves.get(move_.0).unwrap_or(&default_learn);
            let learn_level = if *level == 0i64 {
                "".to_string()
            } else {
                level.to_string()
            };
            let learn = format!("{} {}", learn_method, learn_level);

            writeln!(f, "{prop:40}{stats:80}{learn}")?;
        }

        Ok(())
    }
}

pub struct MoveDisplay<'a, 'b> {
    move_: &'a Move<'b>,
    css: StyleSheet,
}

impl<'a, 'b> MoveDisplay<'a, 'b> {
    pub fn new(move_: &'a Move<'b>) -> Self {
        MoveDisplay {
            move_,
            css: StyleSheet::default(),
        }
    }

    pub fn print(&self) -> Result<()> {
        let Move {
            power,
            accuracy,
            pp,
            name,
            effect,
            damage_class,
            type_,
            ..
        } = self.move_;

        let stats = format!(
            "power: {:3}  accuracy: {:3}  pp: {:3}",
            self.css.power.style(power.unwrap_or(0)),
            self.css.accuracy.style(accuracy.unwrap_or(0)),
            self.css.pp.style(pp.unwrap_or(0))
        );

        let mut f = stdout().lock();
        writedoc! {
            f,
            "

            {name}
            {type_} {damage_class}
            {stats}
            {effect}

            ",
            name = self.css.header.style(name)
        }?;

        Ok(())
    }
}
