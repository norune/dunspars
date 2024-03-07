use super::{Colors, DisplayComponent};
use crate::models::Stats;

use std::fmt;

use indoc::writedoc;

impl fmt::Display for DisplayComponent<&Stats> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Stats {
            hp,
            attack,
            defense,
            special_attack,
            special_defense,
            speed,
        } = self.context;
        let total = hp + attack + defense + special_attack + special_defense + speed;

        // 255 is the actual stat ceiling, but 200 is the ceiling for the vast majority of pokemon
        let hp_color = self.ansi(Colors::rate(*hp, 200));
        let at_color = self.ansi(Colors::rate(*attack, 200));
        let df_color = self.ansi(Colors::rate(*defense, 200));
        let sat_color = self.ansi(Colors::rate(*special_attack, 200));
        let sdf_color = self.ansi(Colors::rate(*special_defense, 200));
        let spd_color = self.ansi(Colors::rate(*speed, 200));
        // 720 is based on Arceus' total stats
        let total_color = self.ansi_bold(Colors::rate(total, 720));

        writedoc! {
            f,
            "hp    atk   def   satk  sdef  spd   total
            {hp_color}{hp:<6}{at_color}{attack:<6}{df_color}{defense:<6}{sat_color}{special_attack:<6}\
            {sdf_color}{special_defense:<6}{spd_color}{speed:<6}{total_color}{total:<6}{total_color:#}",
        }
    }
}
