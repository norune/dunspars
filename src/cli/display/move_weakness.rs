use super::{Colors, DisplayComponent, Effects, WeaknessDisplay};
use crate::cli::utils::is_stab;
use crate::models::{Move, Pokemon, TypeChart};

use std::fmt;

use indoc::writedoc;
use rusqlite::Connection;

pub struct MoveWeaknessComponent<'a> {
    pub defender: &'a Pokemon,
    pub attacker: &'a Pokemon,
    pub db: &'a Connection,
    pub verbose: bool,
    pub stab_only: bool,
}

impl fmt::Display for DisplayComponent<MoveWeaknessComponent<'_>> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let MoveWeaknessComponent {
            defender,
            attacker,
            db,
            verbose,
            stab_only,
        } = self.context;

        let move_list = attacker.get_move_list(db).unwrap();
        let attacker_moves = if move_list.is_empty() {
            attacker.get_learnable_move_list(db).unwrap()
        } else {
            move_list
        };

        let defender_defense = defender.get_defense_chart(db).unwrap();

        let weakness_groups = self.group_by_weakness(attacker_moves.get_list(), |move_| {
            let multiplier = defender_defense.get_multiplier(&move_.1.type_);

            let stab_qualified = !stab_only || is_stab(&move_.1.type_, attacker);
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
            let color = if is_stab(&move_.type_, self.context.attacker) {
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
