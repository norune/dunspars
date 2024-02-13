use std::fmt;

use indoc::writedoc;

use crate::cli::utils::{
    is_color_enabled, rate_number_to_color, Colors, DisplayComponent, Effects, WeaknessDisplay,
};
use crate::pokemon::{
    self, Ability, EvolutionMethod, EvolutionStep, Move, MoveList, Pokemon, PokemonData,
    PokemonGroup, Stats, TypeChart,
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
            game,
            generation,
            primary_type,
            secondary_type,
            group,
            stats,
            abilities,
            ..
        } = self.pokemon;

        let secondary_type = match secondary_type {
            Some(type_) => format!(" {type_} "),
            None => " ".to_string(),
        };

        let group = match group {
            PokemonGroup::Mythical => "mythical",
            PokemonGroup::Legendary => "legendary",
            PokemonGroup::Regular => "",
        };

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
            "{header}{name}{header:#} {primary_type}{secondary_type}{yellow}{group}{yellow:#}
            {abilities}
            {stats_display}
            {game} gen-{generation}",
            header = self.fg_effect(Colors::Header, Effects::Bold),
            yellow = self.fg(Colors::Yellow),
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

        // 255 is the actual stat ceiling, but Blissey/Chansey's HP is the only stat that exceeds 200
        let hp_color = self.fg(rate_number_to_color(*hp as f64, 200f64));
        let at_color = self.fg(rate_number_to_color(*attack as f64, 200f64));
        let df_color = self.fg(rate_number_to_color(*defense as f64, 200f64));
        let sat_color = self.fg(rate_number_to_color(*special_attack as f64, 200f64));
        let sdf_color = self.fg(rate_number_to_color(*special_defense as f64, 200f64));
        let spd_color = self.fg(rate_number_to_color(*speed as f64, 200f64));
        // 720 is based on Arceus' total stats; highest as of this writing
        let total_color = self.fg_effect(rate_number_to_color(total as f64, 720f64), Effects::Bold);

        writedoc! {
            f,
            "hp    atk   def   satk  sdef  spd   total
            {hp_color}{hp:<6}{at_color}{attack:<6}{df_color}{defense:<6}{sat_color}{special_attack:<6}\
            {sdf_color}{special_defense:<6}{spd_color}{speed:<6}{total_color}{total:<6}{total_color:#}",
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
        let weakness_groups = self.group_by_weakness(self.type_chart.get_value(), |item| {
            Some((item.0.clone(), *item.1))
        });
        let type_chart = self.format_groups(weakness_groups);

        writedoc! {
            f,
            "{header}{label}{header:#}{type_chart}",
            header = self.fg_effect(Colors::Header, Effects::Bold),
            label = self.label,
        }
    }
}

impl WeaknessDisplay<String> for TypeChartDisplay<'_> {
    fn format_group(&self, label: &'static str, types: Vec<String>, color: Colors) -> String {
        let style = self.fg(color);
        format!("\n{label}: {style}{}{style:#}", types.join(" "))
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
}

pub struct MoveWeaknessDisplay<'a, 'b> {
    defender: &'a Pokemon<'b>,
    attacker: &'a Pokemon<'b>,
    stab_only: bool,
    color_enabled: bool,
}

impl DisplayComponent for MoveWeaknessDisplay<'_, '_> {
    fn color_enabled(&self) -> bool {
        self.color_enabled
    }
}

impl<'a, 'b> WeaknessDisplay<&'a Move<'b>> for MoveWeaknessDisplay<'a, 'b> {
    fn format_group(&self, label: &'static str, moves: Vec<&'a Move<'b>>, color: Colors) -> String {
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
            let stab_qualified =
                !self.stab_only || pokemon::is_stab(&move_.1.type_, &self.attacker.data);
            if move_.1.damage_class != "status" && stab_qualified {
                let multiplier = self.defender.defense_chart.get_multiplier(&move_.1.type_);
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
    pub fn new(defender: &'a Pokemon<'b>, attacker: &'a Pokemon<'b>, stab_only: bool) -> Self {
        Self {
            defender,
            attacker,
            stab_only,
            color_enabled: is_color_enabled(),
        }
    }
}

pub struct MatchDisplay<'a, 'b> {
    defender: &'a Pokemon<'b>,
    attacker: &'a Pokemon<'b>,
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
        let defender_stats = StatsDisplay::new(&self.defender.data.stats);
        let attacker_stats = StatsDisplay::new(&self.attacker.data.stats);

        let defender_moves_header = format!(
            "{}'s moves vs {}",
            self.attacker.data.name, self.defender.data.name
        );
        let defender_weaknesses =
            MoveWeaknessDisplay::new(self.defender, self.attacker, self.stab_only);

        let attacker_moves_header = format!(
            "{}'s moves vs {}",
            self.defender.data.name, self.attacker.data.name
        );
        let attacker_weaknesses =
            MoveWeaknessDisplay::new(self.attacker, self.defender, self.stab_only);

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

impl<'a, 'b> MatchDisplay<'a, 'b> {
    pub fn new(defender: &'a Pokemon<'b>, attacker: &'a Pokemon<'b>, stab_only: bool) -> Self {
        MatchDisplay {
            defender,
            attacker,
            stab_only,
            color_enabled: is_color_enabled(),
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

            let (name_space, type_space, stats_space) = if self.color_enabled {
                (35, 20, 80)
            } else {
                (21, 20, 37)
            };

            writedoc! {
                f,
                "\n{move_name:name_space$}{move_type:type_space$}{move_stats:stats_space$}{learn_method} {learn_level}",
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
