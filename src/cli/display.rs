use std::fmt;

use indoc::{formatdoc, writedoc};

use crate::cli::utils::{is_color_enabled, Colors, DisplayComponent, Effects, WeaknessGroups};
use crate::pokemon::{
    self, Ability, EvolutionMethod, EvolutionStep, Move, MoveList, Pokemon, PokemonData, Stats,
    TypeChart,
};

pub struct PokemonDisplay<'a, 'b> {
    pokemon: &'a PokemonData<'b>,
    color_enabled: bool,
}

impl DisplayComponent for PokemonDisplay<'_, '_> {
    fn color_enabled(&self) -> bool {
        self.color_enabled
    }
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
            "{header}{name}{header:#} {primary_type} {secondary_type}
            {abilities}
            {stats_display}
            {version} gen-{generation}",
            header = self.fg_effect(Colors::Header, Effects::Bold),
            secondary_type = secondary_type.as_deref().unwrap_or("")
        }
    }
}

impl<'a, 'b> PokemonDisplay<'a, 'b> {
    pub fn new(pokemon: &'a PokemonData<'b>) -> Self {
        Self {
            pokemon,
            color_enabled: is_color_enabled(),
        }
    }
}

pub struct StatsDisplay<'a> {
    stats: &'a Stats,
    color_enabled: bool,
}

impl DisplayComponent for StatsDisplay<'_> {
    fn color_enabled(&self) -> bool {
        self.color_enabled
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
        let total = hp + attack + defense + special_attack + special_defense + speed;

        writedoc! {
            f,
            "hp    atk   def   satk  sdef  spd   total
            {red}{hp:<6}{yellow}{attack:<6}{blue}{defense:<6}{green}{special_attack:<6}\
            {cyan}{special_defense:<6}{violet}{speed:<6}{header}{total:<6}{header:#}",
            red = self.fg(Colors::Red),
            yellow = self.fg(Colors::Yellow),
            blue = self.fg(Colors::Blue),
            green = self.fg(Colors::Green),
            cyan = self.fg(Colors::Cyan),
            violet = self.fg(Colors::Violet),
            header = self.fg_effect(Colors::Header, Effects::Bold),
        }
    }
}

impl<'a> StatsDisplay<'a> {
    pub fn new(stats: &'a Stats) -> Self {
        Self {
            stats,
            color_enabled: is_color_enabled(),
        }
    }
}

pub struct TypeChartDisplay<'a> {
    type_chart: &'a TypeChart,
    label: &'static str,
    color_enabled: bool,
}

impl DisplayComponent for TypeChartDisplay<'_> {
    fn color_enabled(&self) -> bool {
        self.color_enabled
    }
}

impl fmt::Display for TypeChartDisplay<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let weakness_groups =
            WeaknessGroups::new(self.type_chart.get_value(), |t| Some((t.0.clone(), *t.1)));
        let type_chart = self.format_type_chart(weakness_groups);

        writedoc! {
            f,
            "{header}{label}{header:#}{type_chart}",
            header = self.fg_effect(Colors::Header, Effects::Bold),
            label = self.label,
        }
    }
}

impl<'a> TypeChartDisplay<'a> {
    pub fn new(type_chart: &'a TypeChart, label: &'static str) -> Self {
        Self {
            type_chart,
            label,
            color_enabled: is_color_enabled(),
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
            quad = self.format_group("quad", weakness_groups.quad, Colors::Red);
        }
        if !weakness_groups.double.is_empty() {
            double = self.format_group("double", weakness_groups.double, Colors::Yellow);
        }
        if !weakness_groups.neutral.is_empty() {
            neutral = self.format_group("neutral", weakness_groups.neutral, Colors::Green);
        }
        if !weakness_groups.half.is_empty() {
            half = self.format_group("half", weakness_groups.half, Colors::Blue);
        }
        if !weakness_groups.quarter.is_empty() {
            quarter = self.format_group("quarter", weakness_groups.quarter, Colors::Cyan);
        }
        if !weakness_groups.zero.is_empty() {
            zero = self.format_group("zero", weakness_groups.zero, Colors::Violet);
        }
        if !weakness_groups.other.is_empty() {
            other = self.format_group("other", weakness_groups.other, Colors::Green);
        }

        formatdoc! {
            "{quad}{double}{neutral}{half}{quarter}{zero}{other}"
        }
    }

    fn format_group(&self, label: &'static str, types: Vec<String>, color: Colors) -> String {
        let style = self.fg(color);
        format!("\n{label}: {style}{}{style:#}", types.join(" "))
    }
}

pub struct MatchDisplay<'a, 'b, 'c> {
    defender: &'a Pokemon<'c>,
    attacker: &'b Pokemon<'c>,
    stab_only: bool,
    color_enabled: bool,
}

impl DisplayComponent for MatchDisplay<'_, '_, '_> {
    fn color_enabled(&self) -> bool {
        self.color_enabled
    }
}

impl fmt::Display for MatchDisplay<'_, '_, '_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let defender_weakness_group = self.get_weakness_groups(self.defender, self.attacker);
        let attacker_weakness_group = self.get_weakness_groups(self.attacker, self.defender);

        let defender_stats = StatsDisplay::new(&self.defender.data.stats);
        let attacker_stats = StatsDisplay::new(&self.attacker.data.stats);
        let defender_weaknesses =
            self.format_weakness_groups(defender_weakness_group, &self.attacker.data);
        let attacker_weaknesses =
            self.format_weakness_groups(attacker_weakness_group, &self.defender.data);

        let defender_moves_header = format!(
            "{}'s moves vs {}",
            self.attacker.data.name, self.defender.data.name
        );
        let attacker_moves_header = format!(
            "{}'s moves vs {}",
            self.defender.data.name, self.attacker.data.name
        );

        writedoc! {
            f,
            "{header}{defender_header}{header:#} {defender_primary_type} {defender_secondary_type}
            {defender_stats}
            {header}{attacker_header}{header:#} {attacker_primary_type} {attacker_secondary_type}
            {attacker_stats}

            {header}{defender_moves_header}{header:#}{defender_weaknesses}

            {header}{attacker_moves_header}{header:#}{attacker_weaknesses}",
            defender_header = &self.defender.data.name,
            defender_primary_type = self.defender.data.primary_type,
            defender_secondary_type = self.defender.data.secondary_type.as_deref().unwrap_or(""),
            attacker_header = &self.attacker.data.name,
            attacker_primary_type = self.attacker.data.primary_type,
            attacker_secondary_type = self.attacker.data.secondary_type.as_deref().unwrap_or(""),
            header = self.fg_effect(Colors::Header, Effects::Bold),
        }
    }
}

impl<'a, 'b, 'c> MatchDisplay<'a, 'b, 'c> {
    pub fn new(defender: &'a Pokemon<'c>, attacker: &'b Pokemon<'c>, stab_only: bool) -> Self {
        MatchDisplay {
            defender,
            attacker,
            stab_only,
            color_enabled: is_color_enabled(),
        }
    }

    fn get_weakness_groups(
        &self,
        defender: &'a Pokemon,
        attacker: &'b Pokemon,
    ) -> WeaknessGroups<&Move<'_>> {
        WeaknessGroups::new(attacker.move_list.get_map(), |move_| {
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
            quad = self.format_group("quad", weakness_groups.quad, attacker, Colors::Red);
        }
        if !weakness_groups.double.is_empty() {
            double = self.format_group("double", weakness_groups.double, attacker, Colors::Yellow);
        }
        if !weakness_groups.neutral.is_empty() {
            neutral =
                self.format_group("neutral", weakness_groups.neutral, attacker, Colors::Green);
        }
        if !weakness_groups.half.is_empty() {
            half = self.format_group("half", weakness_groups.half, attacker, Colors::Blue);
        }
        if !weakness_groups.quarter.is_empty() {
            quarter = self.format_group("quarter", weakness_groups.quarter, attacker, Colors::Cyan);
        }
        if !weakness_groups.zero.is_empty() {
            zero = self.format_group("zero", weakness_groups.zero, attacker, Colors::Violet);
        }
        if !weakness_groups.other.is_empty() {
            other = self.format_group("other", weakness_groups.other, attacker, Colors::Green);
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
        color: Colors,
    ) -> String {
        let mut output = format!("\n{label}: ");

        let style = self.color().fg(color);
        let normal_color = style.ansi();
        let stab_color = style.effect(Effects::Underline).ansi();

        for move_ in moves {
            let damage_class = match move_.damage_class.as_str() {
                "special" => "s",
                "physical" => "p",
                _ => "?",
            };
            let color = if pokemon::is_stab(&move_.type_, attacker) {
                stab_color
            } else {
                normal_color
            };

            output += &format!(
                "{color}{move_name}({damage_class}){color:#} ",
                move_name = move_.name,
            );
        }

        output
    }
}

pub struct MoveListDisplay<'a, 'b, 'c> {
    move_list: &'a MoveList<'c>,
    pokemon: &'b PokemonData<'c>,
    color_enabled: bool,
}

impl DisplayComponent for MoveListDisplay<'_, '_, '_> {
    fn color_enabled(&self) -> bool {
        self.color_enabled
    }
}

impl fmt::Display for MoveListDisplay<'_, '_, '_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{header}moves{header:#}",
            header = self.fg_effect(Colors::Header, Effects::Bold)
        )?;
        let move_list = self.move_list.get_map();

        if move_list.is_empty() {
            write!(f, "\nThere are no moves to display.\n")?;
        }

        for move_ in move_list {
            let Move {
                name,
                accuracy,
                power,
                pp,
                damage_class,
                type_,
                ..
            } = move_.1;

            let stab = if pokemon::is_stab(&move_.1.type_, self.pokemon) {
                "(s)"
            } else {
                ""
            };

            let power = if let Some(power) = power {
                power.to_string()
            } else {
                "N/A".to_string()
            };
            let accuracy = if let Some(accuracy) = accuracy {
                accuracy.to_string()
            } else {
                "N/A".to_string()
            };
            let pp = if let Some(pp) = pp {
                pp.to_string()
            } else {
                "N/A".to_string()
            };

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

            let move_name = format!(
                "{green}{name}{green:#}{stab}",
                green = self.fg(Colors::Green)
            );
            let move_type = format!("{type_} {damage_class}");
            let move_stats = format!(
                "power: {red}{power:3}{red:#}  accuracy: {green}{accuracy:3}{green:#}  pp: {blue}{pp:2}{blue:#}",
                green = self.fg(Colors::Green),
                red = self.fg(Colors::Red),
                blue = self.fg(Colors::Blue),
            );

            writedoc! {
                f,
                "\n{move_name:35}{move_type:20}{move_stats:80}{learn_method} {learn_level}",
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
            color_enabled: is_color_enabled(),
        }
    }
}

pub struct MoveDisplay<'a, 'b> {
    move_: &'a Move<'b>,
    color_enabled: bool,
}

impl DisplayComponent for MoveDisplay<'_, '_> {
    fn color_enabled(&self) -> bool {
        self.color_enabled
    }
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
            effect_chance,
            type_,
            ..
        } = self.move_;

        let power = if let Some(power) = power {
            power.to_string()
        } else {
            "N/A".to_string()
        };
        let accuracy = if let Some(accuracy) = accuracy {
            accuracy.to_string()
        } else {
            "N/A".to_string()
        };
        let pp = if let Some(pp) = pp {
            pp.to_string()
        } else {
            "N/A".to_string()
        };

        let stats = format!(
            "power: {red}{power:3}{red:#}  accuracy: {green}{accuracy:3}{green:#}  pp: {blue}{pp:3}{blue:#}",
            red = self.fg(Colors::Red),
            green = self.fg(Colors::Green),
            blue = self.fg(Colors::Blue),
        );

        let effect_text = if let Some(chance) = effect_chance {
            effect.replace("$effect_chance", &chance.to_string())
        } else {
            effect.to_string()
        };

        writedoc! {
            f,
            "{header}{name}{header:#}
            {type_} {damage_class}
            {stats}
            {effect_text}",
            header = self.fg_effect(Colors::Header, Effects::Bold)
        }
    }
}

impl<'a, 'b> MoveDisplay<'a, 'b> {
    pub fn new(move_: &'a Move<'b>) -> Self {
        MoveDisplay {
            move_,
            color_enabled: is_color_enabled(),
        }
    }
}

pub struct AbilityDisplay<'a, 'b> {
    ability: &'a Ability<'b>,
    color_enabled: bool,
}

impl DisplayComponent for AbilityDisplay<'_, '_> {
    fn color_enabled(&self) -> bool {
        self.color_enabled
    }
}

impl fmt::Display for AbilityDisplay<'_, '_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let Ability { name, effect, .. } = self.ability;

        writedoc! {
            f,
            "{header}{name}{header:#}
            {effect}",
            header = self.fg_effect(Colors::Header, Effects::Bold)
        }
    }
}

impl<'a, 'b> AbilityDisplay<'a, 'b> {
    pub fn new(ability: &'a Ability<'b>) -> Self {
        AbilityDisplay {
            ability,
            color_enabled: is_color_enabled(),
        }
    }
}

pub struct EvolutionStepDisplay<'a> {
    evolution_step: &'a EvolutionStep,
    color_enabled: bool,
}

impl DisplayComponent for EvolutionStepDisplay<'_> {
    fn color_enabled(&self) -> bool {
        self.color_enabled
    }
}

impl fmt::Display for EvolutionStepDisplay<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(
            f,
            "{header}evolution{header:#}",
            header = self.fg_effect(Colors::Header, Effects::Bold)
        )?;
        self.traverse_dfs(f, self.evolution_step, 0)?;
        Ok(())
    }
}

impl<'a> EvolutionStepDisplay<'a> {
    pub fn new(evolution_step: &'a EvolutionStep) -> Self {
        EvolutionStepDisplay {
            evolution_step,
            color_enabled: is_color_enabled(),
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
            "{indentation}{green}{species}{green:#} {methods}",
            indentation = "  ".repeat(depth),
            green = self.fg(Colors::Green),
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
        let mut output = format!("{blue}{trigger}{blue:#}", blue = self.fg(Colors::Blue));

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
