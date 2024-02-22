mod pokemon_data;
mod stats;
pub mod typechart;

use std::fmt;

use indoc::writedoc;

use crate::cli::utils::{DisplayComponent, WeaknessDisplay};
use crate::pokemon::{
    self, Ability, EvolutionMethod, EvolutionStep, Move, MoveList, Pokemon, PokemonData,
};

pub struct DisplayComponent2<T> {
    context: T,
    color_enabled: bool,
}

impl<T> DisplayComponent2<T> {
    pub fn new(context: T, color_enabled: bool) -> Self {
        Self {
            context,
            color_enabled,
        }
    }

    pub fn style(&self) -> Style {
        Style::new(self.color_enabled)
    }

    pub fn ansi(&self, color: Colors) -> anstyle::Style {
        self.style().fg(color).ansi()
    }

    pub fn ansi_bold(&self, color: Colors) -> anstyle::Style {
        self.style().fg(color).effect(Effects::Bold).ansi()
    }

    #[allow(dead_code)]
    pub fn ansi_underline(&self, color: Colors) -> anstyle::Style {
        self.style().fg(color).effect(Effects::Underline).ansi()
    }
}

#[derive(Debug, PartialEq)]
pub enum Colors {
    Header,
    Red,
    Orange,
    Yellow,
    Green,
    Cyan,
    Blue,
    Violet,
}

impl Colors {
    pub fn rate(number: i64, ceiling: i64) -> Self {
        let number = number as f64;
        let ceiling = ceiling as f64;

        match number {
            number if number > ceiling * 0.83 => Colors::Red,
            number if number > ceiling * 0.66 => Colors::Orange,
            number if number > ceiling * 0.50 => Colors::Yellow,
            number if number > ceiling * 0.33 => Colors::Green,
            number if number > ceiling * 0.16 => Colors::Blue,
            _ => Colors::Violet,
        }
    }

    fn get(&self) -> Option<anstyle::Color> {
        match self {
            Colors::Header => Some(anstyle::Ansi256Color(10).into()),
            Colors::Red => Some(anstyle::Ansi256Color(160).into()),
            Colors::Orange => Some(anstyle::Ansi256Color(172).into()),
            Colors::Yellow => Some(anstyle::Ansi256Color(184).into()),
            Colors::Green => Some(anstyle::Ansi256Color(77).into()),
            Colors::Cyan => Some(anstyle::Ansi256Color(43).into()),
            Colors::Blue => Some(anstyle::Ansi256Color(33).into()),
            Colors::Violet => Some(anstyle::Ansi256Color(99).into()),
        }
    }
}

pub enum Effects {
    Bold,
    Underline,
}

impl Effects {
    fn get(&self) -> anstyle::Effects {
        match self {
            Effects::Bold => anstyle::Effects::BOLD,
            Effects::Underline => anstyle::Effects::UNDERLINE,
        }
    }
}

pub struct Style {
    style: anstyle::Style,
    color_enabled: bool,
}

impl Style {
    pub fn new(color_enabled: bool) -> Self {
        Self {
            style: anstyle::Style::new(),
            color_enabled,
        }
    }

    pub fn fg(mut self, color: Colors) -> Self {
        if self.color_enabled {
            self.style = self.style.fg_color(color.get());
        }
        self
    }

    #[allow(dead_code)]
    pub fn bg(mut self, color: Colors) -> Self {
        if self.color_enabled {
            self.style = self.style.bg_color(color.get());
        }
        self
    }

    pub fn effect(mut self, effect: Effects) -> Self {
        if self.color_enabled {
            self.style = self.style.effects(effect.get());
        }
        self
    }

    pub fn ansi(&self) -> anstyle::Style {
        self.style
    }
}

pub struct MoveWeaknessDisplay<'a, 'b> {
    defender: &'a Pokemon<'b>,
    attacker: &'a Pokemon<'b>,
    verbose: bool,
    stab_only: bool,
    color_enabled: bool,
}

impl DisplayComponent for MoveWeaknessDisplay<'_, '_> {
    fn color_enabled(&self) -> bool {
        self.color_enabled
    }
}

impl<'a, 'b> WeaknessDisplay<&'a Move<'b>> for MoveWeaknessDisplay<'a, 'b> {
    fn format_group(
        &self,
        label: &'static str,
        mut moves: Vec<&'a Move<'b>>,
        color: Colors,
    ) -> String {
        let mut output = format!("\n{label}: ");

        let style = self.style().fg(color);
        let normal_color = style.ansi();
        let stab_color = style.effect(Effects::Underline).ansi();

        moves.sort_by_key(|m| m.name.clone());
        for move_ in moves {
            let damage_class = match move_.damage_class.as_str() {
                "special" => "s",
                "physical" => "p",
                _ => "?",
            };
            let color = if pokemon::is_stab(&move_.type_, &self.attacker.data) {
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

impl fmt::Display for MoveWeaknessDisplay<'_, '_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let weakness_groups = self.group_by_weakness(self.attacker.move_list.get_map(), |move_| {
            let multiplier = self.defender.defense_chart.get_multiplier(&move_.1.type_);

            let stab_qualified =
                !self.stab_only || pokemon::is_stab(&move_.1.type_, &self.attacker.data);
            let verbose_qualified = self.verbose || multiplier >= 2.0;

            if move_.1.damage_class != "status" && stab_qualified && verbose_qualified {
                Some((move_.1, multiplier))
            } else {
                None
            }
        });
        let defender_weaknesses = self.format_groups(weakness_groups);

        writedoc! {
            f,
            "{defender_weaknesses}",
        }
    }
}

impl<'a, 'b> MoveWeaknessDisplay<'a, 'b> {
    pub fn new(
        defender: &'a Pokemon<'b>,
        attacker: &'a Pokemon<'b>,
        verbose: bool,
        stab_only: bool,
        color_enabled: bool,
    ) -> Self {
        Self {
            defender,
            attacker,
            verbose,
            stab_only,
            color_enabled,
        }
    }
}

pub struct MatchDisplay<'a, 'b> {
    defender: &'a Pokemon<'b>,
    attacker: &'a Pokemon<'b>,
    verbose: bool,
    stab_only: bool,
    color_enabled: bool,
}

impl DisplayComponent for MatchDisplay<'_, '_> {
    fn color_enabled(&self) -> bool {
        self.color_enabled
    }
}

impl fmt::Display for MatchDisplay<'_, '_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let defender_stats = DisplayComponent2::new(&self.defender.data.stats, self.color_enabled);
        let attacker_stats = DisplayComponent2::new(&self.attacker.data.stats, self.color_enabled);

        let defender_moves_header = format!(
            "{}'s moves vs {}",
            self.attacker.data.name, self.defender.data.name
        );
        let defender_weaknesses = MoveWeaknessDisplay::new(
            self.defender,
            self.attacker,
            self.verbose,
            self.stab_only,
            self.color_enabled,
        );

        let attacker_moves_header = format!(
            "{}'s moves vs {}",
            self.defender.data.name, self.attacker.data.name
        );
        let attacker_weaknesses = MoveWeaknessDisplay::new(
            self.attacker,
            self.defender,
            self.verbose,
            self.stab_only,
            self.color_enabled,
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
            header = self.color_effect(Colors::Header, Effects::Bold),
        }
    }
}

impl<'a, 'b> MatchDisplay<'a, 'b> {
    pub fn new(
        defender: &'a Pokemon<'b>,
        attacker: &'a Pokemon<'b>,
        verbose: bool,
        stab_only: bool,
        color_enabled: bool,
    ) -> Self {
        MatchDisplay {
            defender,
            attacker,
            verbose,
            stab_only,
            color_enabled,
        }
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
            header = self.color_effect(Colors::Header, Effects::Bold)
        )?;

        let mut move_list = self
            .pokemon
            .learn_moves
            .iter()
            .map(|m| (m.0, &m.1 .0, m.1 .1))
            .collect::<Vec<(&String, &String, i64)>>();

        if move_list.is_empty() {
            write!(f, "\nThere are no moves to display.\n")?;
        } else {
            // Sort by name, then by level, then by method
            move_list.sort_by(|(a_name, a_method, a_level), (b_name, b_method, b_level)| {
                a_method
                    .cmp(b_method)
                    .then(a_level.cmp(b_level))
                    .then(a_name.cmp(b_name))
            });
        }

        for (name, learn_method, learn_level) in move_list {
            let Move {
                name,
                accuracy,
                power,
                pp,
                damage_class,
                type_,
                ..
            } = self.move_list.get_map().get(name).unwrap();

            let stab = if pokemon::is_stab(type_, self.pokemon) {
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

            let level = if learn_level == 0i64 && learn_method == "level-up" {
                "evolve".to_string()
            } else if learn_method == "level-up" {
                learn_level.to_string()
            } else {
                "".to_string()
            };

            let move_name = format!(
                "{green}{name}{green:#}{stab}",
                green = self.color(Colors::Green)
            );
            let move_type = format!("{type_} {damage_class}");
            let move_stats = format!(
                "power: {red}{power:3}{red:#}  accuracy: {green}{accuracy:3}{green:#}  pp: {blue}{pp:2}{blue:#}",
                green = self.color(Colors::Green),
                red = self.color(Colors::Red),
                blue = self.color(Colors::Blue),
            );

            let (name_space, type_space, stats_space) = if self.color_enabled {
                (35, 20, 80)
            } else {
                (21, 20, 37)
            };

            writedoc! {
                f,
                "\n{move_name:name_space$}{move_type:type_space$}{move_stats:stats_space$}{learn_method} {level}",
            }?;
        }

        Ok(())
    }
}

impl<'a, 'b, 'c> MoveListDisplay<'a, 'b, 'c> {
    pub fn new(
        move_list: &'a MoveList<'c>,
        pokemon: &'b PokemonData<'c>,
        color_enabled: bool,
    ) -> Self {
        MoveListDisplay {
            move_list,
            pokemon,
            color_enabled,
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
            red = self.color(Colors::Red),
            green = self.color(Colors::Green),
            blue = self.color(Colors::Blue),
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
            header = self.color_effect(Colors::Header, Effects::Bold)
        }
    }
}

impl<'a, 'b> MoveDisplay<'a, 'b> {
    pub fn new(move_: &'a Move<'b>, color_enabled: bool) -> Self {
        MoveDisplay {
            move_,
            color_enabled,
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
            header = self.color_effect(Colors::Header, Effects::Bold)
        }
    }
}

impl<'a, 'b> AbilityDisplay<'a, 'b> {
    pub fn new(ability: &'a Ability<'b>, color_enabled: bool) -> Self {
        AbilityDisplay {
            ability,
            color_enabled,
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
            header = self.color_effect(Colors::Header, Effects::Bold)
        )?;
        self.traverse_dfs(f, self.evolution_step, 0)?;
        Ok(())
    }
}

impl<'a> EvolutionStepDisplay<'a> {
    pub fn new(evolution_step: &'a EvolutionStep, color_enabled: bool) -> Self {
        EvolutionStepDisplay {
            evolution_step,
            color_enabled,
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
            green = self.color(Colors::Green),
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
        let mut output = format!("{blue}{trigger}{blue:#}", blue = self.color(Colors::Blue));

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
