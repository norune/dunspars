use super::{Colors, DisplayComponent};
use crate::pokemon::Move;

use std::fmt;

use indoc::writedoc;

impl fmt::Display for DisplayComponent<&Move<'_>> {
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
        } = self.context;

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
            red = self.ansi(Colors::Red),
            green = self.ansi(Colors::Green),
            blue = self.ansi(Colors::Blue),
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
            header = self.ansi_bold(Colors::Header)
        }
    }
}
