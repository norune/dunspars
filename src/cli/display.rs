use std::fmt;

use indoc::{formatdoc, writedoc};
use owo_colors::Style;

use crate::cli::utils::{StyleSheet, WeaknessGroups};
use crate::pokemon::{
    self, Ability, EvolutionMethod, EvolutionStep, Move, MoveList, Pokemon, PokemonData, Stats,
    TypeChart,
};

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
            abilities,
            ..
        } = self.pokemon;

        let stats_display = StatsDisplay::new(stats);
        let abilities = abilities
            .iter()
            .map(|a| {
                if a.1 {
                    format!("{}(h)", a.0)
                } else {
                    a.0.clone()
                }
            })
            .collect::<Vec<_>>()
            .join(" ");

        writedoc! {
            f,
            "{name} {primary_type} {secondary_type}
            {abilities}
            {stats_display}
            {version}({generation})",
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
            hp = self.css.accent_red.style(hp),
            attack = self.css.accent_yellow.style(attack),
            defense = self.css.accent_blue.style(defense),
            special_attack = self.css.accent_green.style(special_attack),
            special_defense = self.css.accent_cyan.style(special_defense),
            speed = self.css.accent_violet.style(speed),
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
        let weakness_groups =
            WeaknessGroups::new(self.type_chart.get_value(), |t| Some((t.0.clone(), *t.1)));
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

        if !weakness_groups.quad.is_empty() {
            quad = self.format_group("quad", weakness_groups.quad, self.css.accent_red);
        }
        if !weakness_groups.double.is_empty() {
            double = self.format_group("double", weakness_groups.double, self.css.accent_yellow);
        }
        if !weakness_groups.neutral.is_empty() {
            neutral = self.format_group("neutral", weakness_groups.neutral, self.css.accent_green);
        }
        if !weakness_groups.half.is_empty() {
            half = self.format_group("half", weakness_groups.half, self.css.accent_blue);
        }
        if !weakness_groups.quarter.is_empty() {
            quarter = self.format_group("quarter", weakness_groups.quarter, self.css.accent_cyan);
        }
        if !weakness_groups.zero.is_empty() {
            zero = self.format_group("zero", weakness_groups.zero, self.css.accent_violet);
        }
        if !weakness_groups.other.is_empty() {
            other = self.format_group("other", weakness_groups.other, self.css.accent_green);
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

        if !weakness_groups.quad.is_empty() {
            quad = self.format_group("quad", weakness_groups.quad, attacker, self.css.accent_red);
        }
        if !weakness_groups.double.is_empty() {
            double = self.format_group(
                "double",
                weakness_groups.double,
                attacker,
                self.css.accent_yellow,
            );
        }
        if !weakness_groups.neutral.is_empty() {
            neutral = self.format_group(
                "neutral",
                weakness_groups.neutral,
                attacker,
                self.css.accent_green,
            );
        }
        if !weakness_groups.half.is_empty() {
            half = self.format_group("half", weakness_groups.half, attacker, self.css.accent_blue);
        }
        if !weakness_groups.quarter.is_empty() {
            quarter = self.format_group(
                "quarter",
                weakness_groups.quarter,
                attacker,
                self.css.accent_cyan,
            );
        }
        if !weakness_groups.zero.is_empty() {
            zero = self.format_group(
                "zero",
                weakness_groups.zero,
                attacker,
                self.css.accent_violet,
            );
        }
        if !weakness_groups.other.is_empty() {
            other = self.format_group(
                "other",
                weakness_groups.other,
                attacker,
                self.css.accent_green,
            );
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
            let styled_move = if pokemon::is_stab(&move_.type_, attacker) {
                group_style.underline().style(move_string)
            } else {
                group_style.style(move_string)
            };

            output += &format!("{} ", styled_move);
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
        write!(f, "{}", self.css.header.style("moves"))?;

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

            let move_name = format!("{name}{stab}", name = self.css.accent_green.style(name));
            let type_damage = format!("{type_} {damage_class}");
            let stats = format!(
                "power: {:3}  accuracy: {:3}  pp: {:2}",
                self.css.accent_red.style(power.unwrap_or(0)),
                self.css.accent_green.style(accuracy.unwrap_or(0)),
                self.css.accent_blue.style(pp.unwrap_or(0))
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
                "\n{move_name:33}{type_damage:22}{stats:80}{learn}",
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
            self.css.accent_red.style(power.unwrap_or(0)),
            self.css.accent_green.style(accuracy.unwrap_or(0)),
            self.css.accent_blue.style(pp.unwrap_or(0))
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

pub struct AbilityDisplay<'a, 'b> {
    ability: &'a Ability<'b>,
    css: StyleSheet,
}

impl fmt::Display for AbilityDisplay<'_, '_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let Ability { name, effect, .. } = self.ability;

        writedoc! {
            f,
            "{name}
            {effect}",
            name = self.css.header.style(name),
        }
    }
}

impl<'a, 'b> AbilityDisplay<'a, 'b> {
    pub fn new(ability: &'a Ability<'b>) -> Self {
        AbilityDisplay {
            ability,
            css: StyleSheet::default(),
        }
    }
}

pub struct EvolutionStepDisplay<'a> {
    evolution_step: &'a EvolutionStep,
    css: StyleSheet,
}

impl fmt::Display for EvolutionStepDisplay<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.traverse_dfs(f, self.evolution_step, 0)?;
        Ok(())
    }
}

impl<'a> EvolutionStepDisplay<'a> {
    pub fn new(evolution_step: &'a EvolutionStep) -> Self {
        EvolutionStepDisplay {
            evolution_step,
            css: StyleSheet::default(),
        }
    }

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
            "{indentation}{species} {methods}",
            indentation = "  ".repeat(depth),
            species = self.css.header.style(&step.name),
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
        let mut output = format!("{}", self.css.accent_blue.style(trigger));

        if let Some(item) = item {
            output += &format!(" {item}");
        }

        if let Some(gender) = gender {
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
