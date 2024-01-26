use std::fmt;
use std::io::{stdout, Write};

use anyhow::Result;
use indoc::writedoc;
use owo_colors::{colors::xterm, Style};

use crate::pokemon::{self, Move, MoveList, Pokemon, Stats, TypeChart};

pub struct StyleSheet {
    pub default: Style,
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
    pub hp: Style,
    pub attack: Style,
    pub defense: Style,
    pub special_attack: Style,
    pub special_defense: Style,
    pub speed: Style,
    pub move_: Style,
}

impl Default for StyleSheet {
    fn default() -> Self {
        Self {
            default: Style::new().default_color(),
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
            hp: Style::new().fg::<xterm::GuardsmanRed>(),
            attack: Style::new().fg::<xterm::DecoOrange>(),
            defense: Style::new().fg::<xterm::AeroBlue>(),
            special_attack: Style::new().fg::<xterm::BlazeOrange>(),
            special_defense: Style::new().fg::<xterm::PoloBlue>(),
            speed: Style::new().fg::<xterm::PurplePizzazz>(),
            move_: Style::new().fg::<xterm::SpringGreen>(),
        }
    }
}

pub struct PokemonDisplay<'a, 'b> {
    pokemon: &'a Pokemon<'b>,
    css: StyleSheet,
}

impl<'a, 'b> PokemonDisplay<'a, 'b> {
    pub fn new(pokemon: &'a Pokemon<'b>) -> Self {
        Self {
            pokemon,
            css: StyleSheet::default(),
        }
    }

    pub fn print(&self) -> Result<()> {
        let Pokemon {
            name,
            version,
            generation,
            primary_type,
            secondary_type,
            stats,
            ..
        } = self.pokemon;

        let stats_display = StatsDisplay::new(stats);

        let mut f = stdout().lock();
        writedoc! {
            f,
            "

            {name} {version}({generation})
            {primary_type} {secondary_type}
            {stats_display}
            ",
            name = self.css.header.style(name),
            secondary_type = secondary_type.as_ref().unwrap_or(&"".to_string())
        }?;
        Ok(())
    }
}

pub struct StatsDisplay<'a> {
    stats: &'a Stats,
    css: StyleSheet,
}

impl<'a> StatsDisplay<'a> {
    pub fn new(stats: &'a Stats) -> Self {
        Self {
            stats,
            css: StyleSheet::default(),
        }
    }
}

impl fmt::Display for StatsDisplay<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let Stats {
            hp,
            attack,
            defense,
            special_attack,
            special_defense,
            speed,
        } = self.stats;

        writedoc! {
            f,
            "
            hp    atk   def   satk  sdef  spd
            {hp:<6}{attack:<6}{defense:<6}{special_attack:<6}{special_defense:<6}{speed:<6}            
            ",
            hp = self.css.hp.style(hp),
            attack = self.css.attack.style(attack),
            defense = self.css.defense.style(defense),
            special_attack = self.css.special_attack.style(special_attack),
            special_defense = self.css.special_defense.style(special_defense),
            speed = self.css.speed.style(speed),
        }
    }
}

pub struct TypeChartDisplay<'a> {
    type_chart: &'a TypeChart,
    label: &'static str,
    css: StyleSheet,
}

impl<'a> TypeChartDisplay<'a> {
    pub fn new(type_chart: &'a TypeChart, label: &'static str) -> Self {
        TypeChartDisplay {
            type_chart,
            label,
            css: StyleSheet::default(),
        }
    }

    pub fn print(&self) -> Result<()> {
        let weakness_groups = WeaknessGroups::new(self.type_chart.get_value(), |t| {
            Some((t.0.clone(), t.1.clone()))
        });

        println!("\n{}", self.css.header.style(self.label));
        self.print_weakness_groups(weakness_groups)
    }

    fn print_weakness_groups(&self, weakness_groups: WeaknessGroups<String>) -> Result<()> {
        let mut f = stdout().lock();

        if weakness_groups.quad.len() > 0 {
            let quad = weakness_groups.quad.join(" ");
            writeln!(f, "quad:\n{}", self.css.quad.style(quad))?;
        }
        if weakness_groups.double.len() > 0 {
            let double = weakness_groups.double.join(" ");
            writeln!(f, "double:\n{}", self.css.double.style(double))?;
        }
        if weakness_groups.neutral.len() > 0 {
            let neutral = weakness_groups.neutral.join(" ");
            writeln!(f, "neutral:\n{}", self.css.neutral.style(neutral))?;
        }
        if weakness_groups.half.len() > 0 {
            let half = weakness_groups.half.join(" ");
            writeln!(f, "half:\n{}", self.css.half.style(half))?;
        }
        if weakness_groups.quarter.len() > 0 {
            let quarter = weakness_groups.quarter.join(" ");
            writeln!(f, "quarter:\n{}", self.css.quarter.style(quarter))?;
        }
        if weakness_groups.zero.len() > 0 {
            let zero = weakness_groups.zero.join(" ");
            writeln!(f, "zero:\n{}", self.css.zero.style(zero))?;
        }
        if weakness_groups.other.len() > 0 {
            let other = weakness_groups.other.join(" ");
            writeln!(f, "zero:\n{}", other)?;
        }

        Ok(())
    }
}

pub struct MatchDisplay<'a, 'b, 'c, 'd> {
    type_chart: &'a TypeChart,
    move_list: &'b MoveList<'d>,
    defender: &'c Pokemon<'d>,
    attacker: &'c Pokemon<'d>,
    stab_only: bool,
    css: StyleSheet,
}

impl<'a, 'b, 'c, 'd> MatchDisplay<'a, 'b, 'c, 'd> {
    pub fn new(
        type_chart: &'a TypeChart,
        move_list: &'b MoveList<'d>,
        defender: &'c Pokemon<'d>,
        attacker: &'c Pokemon<'d>,
        stab_only: bool,
    ) -> Self {
        MatchDisplay {
            type_chart,
            move_list,
            defender,
            attacker,
            stab_only,
            css: StyleSheet::default(),
        }
    }

    pub fn print(&self) -> Result<()> {
        self.print_stats()?;

        let weakness_groups = WeaknessGroups::new(self.move_list.get_value(), |move_| {
            let stab_qualified = !self.stab_only || pokemon::is_stab(&move_.1.type_, self.attacker);
            if move_.1.damage_class != "status" && stab_qualified {
                let multiplier = self.type_chart.get_multiplier(&move_.1.type_);
                Some((move_.1, multiplier))
            } else {
                None
            }
        });

        let header = format!("{}'s moves vs {}", self.attacker.name, self.defender.name);
        println!("\n{}", self.css.header.style(header));

        self.print_weakness_groups(weakness_groups)
    }

    fn print_stats(&self) -> Result<()> {
        let defender_stats = StatsDisplay::new(&self.defender.stats);
        let attacker_stats = StatsDisplay::new(&self.attacker.stats);

        let mut f = stdout().lock();
        let defender_header = format!("{}'s stats", self.defender.name);
        let attacker_header = format!("{}'s stats", self.attacker.name);
        writedoc! {
            f,
            "
            {defender_header}
            {defender_stats}
            {attacker_header}
            {attacker_stats}
            ",
            defender_header = self.css.header.style(defender_header),
            attacker_header = self.css.header.style(attacker_header),
        }?;

        Ok(())
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
            if pokemon::is_stab(&move_.type_, self.attacker) {
                styled_move = group_style.underline().style(move_string);
            } else {
                styled_move = group_style.style(move_string);
            }

            write!(f, "{} ", styled_move)?;
        }
        writeln!(f, "{}", "\n")?;
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
        println!("\n{}", self.css.header.style("moves"));

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

            let is_stab = pokemon::is_stab(&move_.1.type_, self.pokemon);
            let stab = if is_stab { "(s)" } else { "" };

            let move_name = format!("{name}{stab}", name = self.css.move_.style(name));
            let type_damage = format!("{type_} {damage_class}");
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

            writedoc! {
                f,
                "
                {move_name:33}{type_damage:22}{stats:80}{learn}
                ",
            }?;
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
