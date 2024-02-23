use super::{Colors, DisplayComponent2, MoveWeaknessContext};
use crate::pokemon::Pokemon;

use std::fmt;

use indoc::writedoc;

pub struct MatchContext<'a, 'b> {
    pub defender: &'a Pokemon<'b>,
    pub attacker: &'a Pokemon<'b>,
    pub verbose: bool,
    pub stab_only: bool,
}

impl fmt::Display for DisplayComponent2<MatchContext<'_, '_>> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let MatchContext {
            defender,
            attacker,
            verbose,
            stab_only,
        } = self.context;

        let defender_stats = DisplayComponent2::new(&defender.data.stats, self.color_enabled);
        let attacker_stats = DisplayComponent2::new(&attacker.data.stats, self.color_enabled);

        let defender_moves_header =
            format!("{}'s moves vs {}", attacker.data.name, defender.data.name);
        let defender_context = MoveWeaknessContext {
            defender,
            attacker,
            verbose,
            stab_only,
        };
        let defender_weaknesses = DisplayComponent2::new(defender_context, self.color_enabled);

        let attacker_moves_header =
            format!("{}'s moves vs {}", defender.data.name, attacker.data.name);
        let attacker_context = MoveWeaknessContext {
            defender: attacker,
            attacker: defender,
            verbose,
            stab_only,
        };
        let attacker_weaknesses = DisplayComponent2::new(attacker_context, self.color_enabled);

        writedoc! {
            f,
            "{header}{defender_header}{header:#} {defender_primary_type} {defender_secondary_type}
            {defender_stats}
            {header}{attacker_header}{header:#} {attacker_primary_type} {attacker_secondary_type}
            {attacker_stats}

            {header}{defender_moves_header}{header:#}{defender_weaknesses}

            {header}{attacker_moves_header}{header:#}{attacker_weaknesses}",
            defender_header = &defender.data.name,
            defender_primary_type = defender.data.primary_type,
            defender_secondary_type = defender.data.secondary_type.as_deref().unwrap_or(""),
            attacker_header = &attacker.data.name,
            attacker_primary_type = attacker.data.primary_type,
            attacker_secondary_type = attacker.data.secondary_type.as_deref().unwrap_or(""),
            header = self.ansi_bold(Colors::Header),
        }
    }
}
