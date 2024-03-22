use super::{Colors, DisplayComponent, MoveWeaknessComponent};
use crate::models::Pokemon;

use std::fmt;

use indoc::writedoc;
use rusqlite::Connection;

pub struct MatchComponent<'a> {
    pub defender: &'a Pokemon,
    pub attacker: &'a Pokemon,
    pub db: &'a Connection,
    pub verbose: bool,
    pub stab_only: bool,
}

impl fmt::Display for DisplayComponent<MatchComponent<'_>> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let MatchComponent {
            defender,
            attacker,
            db,
            verbose,
            stab_only,
        } = self.context;

        let defender_stats = DisplayComponent::new(&defender.stats, self.color_enabled);
        let attacker_stats = DisplayComponent::new(&attacker.stats, self.color_enabled);

        let defender_moves_header = format!("{}'s moves vs {}", attacker.name, defender.name);
        let defender_context = MoveWeaknessComponent {
            defender,
            attacker,
            db,
            verbose,
            stab_only,
        };
        let defender_weaknesses = DisplayComponent::new(defender_context, self.color_enabled);

        let attacker_moves_header = format!("{}'s moves vs {}", defender.name, attacker.name);
        let attacker_context = MoveWeaknessComponent {
            defender: attacker,
            attacker: defender,
            db,
            verbose,
            stab_only,
        };
        let attacker_weaknesses = DisplayComponent::new(attacker_context, self.color_enabled);

        writedoc! {
            f,
            "{header}{defender_header}{header:#} {defender_primary_type} {defender_secondary_type}
            {defender_stats}
            {header}{attacker_header}{header:#} {attacker_primary_type} {attacker_secondary_type}
            {attacker_stats}

            {header}{defender_moves_header}{header:#}{defender_weaknesses}

            {header}{attacker_moves_header}{header:#}{attacker_weaknesses}",
            defender_header = &defender.name,
            defender_primary_type = defender.primary_type,
            defender_secondary_type = defender.secondary_type.as_deref().unwrap_or(""),
            attacker_header = &attacker.name,
            attacker_primary_type = attacker.primary_type,
            attacker_secondary_type = attacker.secondary_type.as_deref().unwrap_or(""),
            header = self.ansi_bold(Colors::Header),
        }
    }
}
