use std::fmt;

use indoc::{formatdoc, writedoc};
use owo_colors::Style;

use crate::cli::utils::{StyleSheet, WeaknessGroups};
use crate::pokemon::{self, Move, MoveList, Pokemon, PokemonData, Stats, TypeChart};

pub struct PokemonDisplay<'a, 'b> {
    pokemon: &'a PokemonData<'b>,
    css: StyleSheet,
}

impl fmt::Display for PokemonDisplay<'_, '_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let PokemonData {
            name,
            game: version,
            generation,
            primary_type,
            secondary_type,
            stats,
            ..
        } = self.pokemon;

        let stats_display = StatsDisplay::new(stats);

        writedoc! {
            f,
            "{name} {version}({generation})
            {primary_type} {secondary_type}
            {stats_display}",
            name = self.css.header.style(name),
            secondary_type = secondary_type.as_ref().unwrap_or(&"".to_string())
        }
    }
}

impl<'a, 'b> PokemonDisplay<'a, 'b> {
    pub fn new(pokemon: &'a PokemonData<'b>) -> Self {
        Self {
            pokemon,
            css: StyleSheet::default(),
        }
    }
}

pub struct StatsDisplay<'a> {
    stats: &'a Stats,
    css: StyleSheet,
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
            "hp    atk   def   satk  sdef  spd
            {hp:<6}{attack:<6}{defense:<6}{special_attack:<6}{special_defense:<6}{speed:<6}",
            hp = self.css.hp.style(hp),
            attack = self.css.attack.style(attack),
            defense = self.css.defense.style(defense),
            special_attack = self.css.special_attack.style(special_attack),
            special_defense = self.css.special_defense.style(special_defense),
            speed = self.css.speed.style(speed),
        }
    }
}

impl<'a> StatsDisplay<'a> {
    pub fn new(stats: &'a Stats) -> Self {
        Self {
            stats,
            css: StyleSheet::default(),
        }
    }
}

pub struct TypeChartDisplay<'a> {
    type_chart: &'a TypeChart,
    label: &'static str,
    css: StyleSheet,
}

impl fmt::Display for TypeChartDisplay<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let weakness_groups = WeaknessGroups::new(self.type_chart.get_value(), |t| {
            Some((t.0.clone(), t.1.clone()))
        });
        let type_chart = self.format_type_chart(weakness_groups);

        writedoc! {
            f,
            "{header}{type_chart}",
            header = self.css.header.style(self.label)
        }
    }
}

impl<'a> TypeChartDisplay<'a> {
    pub fn new(type_chart: &'a TypeChart, label: &'static str) -> Self {
        TypeChartDisplay {
            type_chart,
            label,
            css: StyleSheet::default(),
        }
    }

    fn format_type_chart(&self, weakness_groups: WeaknessGroups<String>) -> String {
        let mut quad = String::from("");
        let mut double = String::from("");
        let mut neutral = String::from("");
        let mut half = String::from("");
        let mut quarter = String::from("");
        let mut zero = String::from("");
        let mut other = String::from("");

        if weakness_groups.quad.len() > 0 {
            quad = self.format_group("quad", weakness_groups.quad, self.css.quad);
        }
        if weakness_groups.double.len() > 0 {
            double = self.format_group("double", weakness_groups.double, self.css.double);
        }
        if weakness_groups.neutral.len() > 0 {
            neutral = self.format_group("neutral", weakness_groups.neutral, self.css.neutral);
        }
        if weakness_groups.half.len() > 0 {
            half = self.format_group("half", weakness_groups.half, self.css.half);
        }
        if weakness_groups.quarter.len() > 0 {
            quarter = self.format_group("quarter", weakness_groups.quarter, self.css.quarter);
        }
        if weakness_groups.zero.len() > 0 {
            zero = self.format_group("zero", weakness_groups.zero, self.css.zero);
        }
        if weakness_groups.other.len() > 0 {
            other = self.format_group("other", weakness_groups.other, self.css.neutral);
        }

        formatdoc! {
            "{quad}{double}{neutral}{half}{quarter}{zero}{other}"
        }
    }

    fn format_group(&self, label: &'static str, types: Vec<String>, group_style: Style) -> String {
        format!("\n{label}: {}", group_style.style(types.join(" ")))
    }
}

pub struct MatchDisplay<'a, 'b, 'c> {
    defender: &'a Pokemon<'c>,
    attacker: &'b Pokemon<'c>,
    stab_only: bool,
    css: StyleSheet,
}

impl fmt::Display for MatchDisplay<'_, '_, '_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let defender_weakness_group = self.get_weakness_groups(self.defender, self.attacker);
        let attacker_weakness_group = self.get_weakness_groups(self.attacker, self.defender);

        let stats = self.format_stats();
        let defender_weaknesses =
            self.format_weakness_groups(defender_weakness_group, &self.attacker.data);
        let attacker_weaknesses =
            self.format_weakness_groups(attacker_weakness_group, &self.defender.data);

        let stats_header = format!("{} vs {}", self.defender.data.name, self.attacker.data.name);
        let defender_header = format!(
            "{}'s moves vs {}",
            self.attacker.data.name, self.defender.data.name
        );
        let attacker_header = format!(
            "{}'s moves vs {}",
            self.defender.data.name, self.attacker.data.name
        );

        writedoc! {
            f,
            "{stats_header}
            {stats}

            {defender_header}{defender_weaknesses}

            {attacker_header}{attacker_weaknesses}",
            stats_header = self.css.header.style(stats_header),
            defender_header = self.css.header.style(defender_header),
            attacker_header = self.css.header.style(attacker_header),
        }
    }
}

impl<'a, 'b, 'c> MatchDisplay<'a, 'b, 'c> {
    pub fn new(defender: &'a Pokemon<'c>, attacker: &'b Pokemon<'c>, stab_only: bool) -> Self {
        MatchDisplay {
            defender,
            attacker,
            stab_only,
            css: StyleSheet::default(),
        }
    }

    fn format_stats(&self) -> String {
        let defender_stats = StatsDisplay::new(&self.defender.data.stats);
        let attacker_stats = StatsDisplay::new(&self.attacker.data.stats);
        formatdoc! {
            "{defender_stats}
            {attacker_stats}"
        }
    }

    fn get_weakness_groups(
        &self,
        defender: &'a Pokemon,
        attacker: &'b Pokemon,
    ) -> WeaknessGroups<&Move<'_>> {
        WeaknessGroups::new(attacker.move_list.get_value(), |move_| {
            let stab_qualified =
                !self.stab_only || pokemon::is_stab(&move_.1.type_, &attacker.data);
            if move_.1.damage_class != "status" && stab_qualified {
                let multiplier = defender.defense_chart.get_multiplier(&move_.1.type_);
                Some((move_.1, multiplier))
            } else {
                None
            }
        })
    }

    fn format_weakness_groups(
        &self,
        weakness_groups: WeaknessGroups<&Move>,
        attacker: &PokemonData,
    ) -> String {
        let mut quad = String::from("");
        let mut double = String::from("");
        let mut neutral = String::from("");
        let mut half = String::from("");
        let mut quarter = String::from("");
        let mut zero = String::from("");
        let mut other = String::from("");

        if weakness_groups.quad.len() > 0 {
            quad = self.format_group("quad", weakness_groups.quad, attacker, self.css.quad);
        }
        if weakness_groups.double.len() > 0 {
            double = self.format_group("double", weakness_groups.double, attacker, self.css.double);
        }
        if weakness_groups.neutral.len() > 0 {
            neutral = self.format_group(
                "neutral",
                weakness_groups.neutral,
                attacker,
                self.css.neutral,
            );
        }
        if weakness_groups.half.len() > 0 {
            half = self.format_group("half", weakness_groups.half, attacker, self.css.half);
        }
        if weakness_groups.quarter.len() > 0 {
            quarter = self.format_group(
                "quarter",
                weakness_groups.quarter,
                attacker,
                self.css.quarter,
            );
        }
        if weakness_groups.zero.len() > 0 {
            zero = self.format_group("zero", weakness_groups.zero, attacker, self.css.zero);
        }
        if weakness_groups.other.len() > 0 {
            other = self.format_group("other", weakness_groups.other, attacker, self.css.neutral);
        }

        formatdoc! {
            "{quad}{double}{neutral}{half}{quarter}{zero}{other}"
        }
    }

    fn format_group(
        &self,
        label: &'static str,
        moves: Vec<&Move>,
        attacker: &PokemonData,
        group_style: Style,
    ) -> String {
        let mut output = format!("\n{label}: ");

        for move_ in moves {
            let damage_class = match move_.damage_class.as_str() {
                "special" => "s",
                "physical" => "p",
                _ => "?",
            };
            let move_string = format!("{}({})", move_.name, damage_class);
            let styled_move;
            if pokemon::is_stab(&move_.type_, attacker) {
                styled_move = group_style.underline().style(move_string);
            } else {
                styled_move = group_style.style(move_string);
            }

            output.push_str(format!("{} ", styled_move).as_str());
        }

        output
    }
}

pub struct MoveListDisplay<'a, 'b, 'c> {
    move_list: &'a MoveList<'c>,
    pokemon: &'b PokemonData<'c>,
    css: StyleSheet,
}

impl fmt::Display for MoveListDisplay<'_, '_, '_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}\n", self.css.header.style("moves"))?;

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
            let (learn_method, level) = self
                .pokemon
                .learn_moves
                .get(move_.0)
                .unwrap_or(&default_learn);
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

impl<'a, 'b, 'c> MoveListDisplay<'a, 'b, 'c> {
    pub fn new(move_list: &'a MoveList<'c>, pokemon: &'b PokemonData<'c>) -> Self {
        MoveListDisplay {
            move_list,
            pokemon,
            css: StyleSheet::default(),
        }
    }
}

pub struct MoveDisplay<'a, 'b> {
    move_: &'a Move<'b>,
    css: StyleSheet,
}

impl fmt::Display for MoveDisplay<'_, '_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
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

        writedoc! {
            f,
            "{name}
            {type_} {damage_class}
            {stats}
            {effect}",
            name = self.css.header.style(name)
        }
    }
}

impl<'a, 'b> MoveDisplay<'a, 'b> {
    pub fn new(move_: &'a Move<'b>) -> Self {
        MoveDisplay {
            move_,
            css: StyleSheet::default(),
        }
    }
}
