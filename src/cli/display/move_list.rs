use super::{Colors, DisplayComponent};
use crate::cli::utils::is_stab;
use crate::models::{Move, MoveList, PokemonData};

use std::fmt;

use indoc::writedoc;

pub struct MoveListComponent<'a> {
    pub move_list: &'a MoveList,
    pub pokemon: &'a PokemonData,
}

impl fmt::Display for DisplayComponent<MoveListComponent<'_>> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{header}moves{header:#}",
            header = self.ansi_bold(Colors::Header)
        )?;

        let MoveListComponent { pokemon, move_list } = self.context;
        let mut learn_moves = pokemon.learn_moves.clone();

        if learn_moves.is_empty() {
            write!(f, "\nThere are no moves to display.\n")?;
        } else {
            // Sort by name, then by level, then by method
            learn_moves.sort_by(|(a_name, a_method, a_level), (b_name, b_method, b_level)| {
                a_method
                    .cmp(b_method)
                    .then(a_level.cmp(b_level))
                    .then(a_name.cmp(b_name))
            });
        }

        for (name, learn_method, learn_level) in learn_moves {
            let Move {
                name,
                accuracy,
                power,
                pp,
                damage_class,
                type_,
                ..
            } = move_list.get_map().get(&name).unwrap();

            let stab = if is_stab(type_, pokemon) { "(s)" } else { "" };

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
                green = self.ansi(Colors::Green)
            );
            let move_type = format!("{type_} {damage_class}");
            let move_stats = format!(
                "power: {red}{power:3}{red:#}  accuracy: {green}{accuracy:3}{green:#}  pp: {blue}{pp:2}{blue:#}",
                green = self.ansi(Colors::Green),
                red = self.ansi(Colors::Red),
                blue = self.ansi(Colors::Blue),
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
