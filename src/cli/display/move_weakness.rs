use super::{Colors, DisplayComponent, Effects, WeaknessDisplay};
use crate::data::{self, Move, Pokemon, TypeChart};

use std::fmt;

use indoc::writedoc;

pub struct MoveWeaknessComponent<'a> {
    pub defender: &'a Pokemon,
    pub attacker: &'a Pokemon,
    pub verbose: bool,
    pub stab_only: bool,
}

impl fmt::Display for DisplayComponent<MoveWeaknessComponent<'_>> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let MoveWeaknessComponent {
            defender,
            attacker,
            verbose,
            stab_only,
        } = self.context;

        let weakness_groups = self.group_by_weakness(attacker.move_list.get_map(), |move_| {
            let multiplier = defender.defense_chart.get_multiplier(&move_.1.type_);

            let stab_qualified = !stab_only || data::is_stab(&move_.1.type_, &attacker.data);
            let verbose_qualified = verbose || multiplier >= 2.0;

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

impl WeaknessDisplay<&Move> for DisplayComponent<MoveWeaknessComponent<'_>> {
    fn format_group(&self, label: &'static str, mut moves: Vec<&Move>, color: Colors) -> String {
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
            let color = if data::is_stab(&move_.type_, &self.context.attacker.data) {
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
