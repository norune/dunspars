use super::{Colors, DisplayComponent};
use crate::models::Pokemon;

use std::fmt;

use indoc::writedoc;

impl fmt::Display for DisplayComponent<&Pokemon> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let Pokemon {
            name,
            generation,
            primary_type,
            secondary_type,
            group,
            stats,
            abilities,
            ..
        } = self.context;

        let secondary_type = match secondary_type {
            Some(type_) => format!(" {type_} "),
            None => " ".to_string(),
        };

        let stats_display = DisplayComponent::new(stats, self.color_enabled);
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
            gen-{generation}",
            header = self.ansi_bold(Colors::Header),
            yellow = self.ansi(Colors::Yellow),
        }
    }
}
