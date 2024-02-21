use anyhow::Result;

use rustemon::client::RustemonClient;
use rustemon::games::version_group as rustemon_version;
use rustemon::moves::move_ as rustemon_moves;
use rustemon::pokemon::{
    ability as rustemon_ability, pokemon as rustemon_pokemon, type_ as rustemon_type,
};

use rustemon::model::evolution::{
    ChainLink as RustemonEvoStep, EvolutionDetail as RustemonEvoMethod,
};
use rustemon::model::moves::PastMoveStatValues as RustemonPastMoveStats;
use rustemon::model::pokemon::{
    AbilityEffectChange as RustemonPastAbilityEffect, PokemonStat as RustemonStat,
    PokemonType as RustemonTypeSlot, PokemonTypePast as RustemonPastPokemonType,
    TypeRelations as RustemonTypeRelations, TypeRelationsPast as RustemonPastTypeRelations,
};
use rustemon::model::resource::Effect as RustemonEffect;

use super::resource::GetGeneration;
use crate::pokemon::{EvolutionMethod, EvolutionStep, Stats};

pub trait Past<T> {
    fn generation(&self, resource: &impl GetGeneration) -> u8;
    fn value(&self) -> T;
}

impl Past<Vec<RustemonTypeSlot>> for RustemonPastPokemonType {
    fn generation(&self, resource: &impl GetGeneration) -> u8 {
        resource.get_gen_from_url(&self.generation.url)
    }

    fn value(&self) -> Vec<RustemonTypeSlot> {
        self.types.clone()
    }
}

impl Past<RustemonTypeRelations> for RustemonPastTypeRelations {
    fn generation(&self, resource: &impl GetGeneration) -> u8 {
        resource.get_gen_from_url(&self.generation.url)
    }

    fn value(&self) -> RustemonTypeRelations {
        self.damage_relations.clone()
    }
}

impl Past<RustemonPastMoveStats> for RustemonPastMoveStats {
    fn generation(&self, resource: &impl GetGeneration) -> u8 {
        resource.get_gen(&self.version_group.name) - 1
    }

    fn value(&self) -> RustemonPastMoveStats {
        self.clone()
    }
}

impl Past<Vec<RustemonEffect>> for RustemonPastAbilityEffect {
    fn generation(&self, resource: &impl GetGeneration) -> u8 {
        resource.get_gen(&self.version_group.name) - 1
    }

    fn value(&self) -> Vec<RustemonEffect> {
        self.effect_entries.clone()
    }
}

pub fn match_past<T: Past<U>, U>(
    current_generation: u8,
    pasts: &[T],
    generation_resource: &impl GetGeneration,
) -> Option<U> {
    let mut oldest_value = None;
    let mut oldest_generation = 255;

    for past in pasts {
        let past_generation = past.generation(generation_resource);
        if current_generation <= past_generation && past_generation <= oldest_generation {
            oldest_value = Some(past.value());
            oldest_generation = past_generation;
        }
    }

    oldest_value
}

pub async fn get_all_pokemon(client: &RustemonClient) -> Result<Vec<String>> {
    Ok(rustemon_pokemon::get_all_entries(client)
        .await?
        .into_iter()
        .map(|p| p.name)
        .collect::<Vec<String>>())
}

pub async fn get_all_types(client: &RustemonClient) -> Result<Vec<String>> {
    Ok(rustemon_type::get_all_entries(client)
        .await?
        .into_iter()
        .map(|p| p.name)
        .collect::<Vec<String>>())
}

pub async fn get_all_moves(client: &RustemonClient) -> Result<Vec<String>> {
    Ok(rustemon_moves::get_all_entries(client)
        .await?
        .into_iter()
        .map(|p| p.name)
        .collect::<Vec<String>>())
}

pub async fn get_all_abilities(client: &RustemonClient) -> Result<Vec<String>> {
    Ok(rustemon_ability::get_all_entries(client)
        .await?
        .into_iter()
        .map(|p| p.name)
        .collect::<Vec<String>>())
}

pub async fn get_all_games(client: &RustemonClient) -> Result<Vec<String>> {
    Ok(rustemon_version::get_all_entries(client)
        .await?
        .into_iter()
        .map(|p| p.name)
        .collect::<Vec<String>>())
}

impl From<RustemonEvoStep> for EvolutionStep {
    fn from(chain_link: RustemonEvoStep) -> Self {
        let evolution_methods = chain_link
            .evolution_details
            .into_iter()
            .map(EvolutionMethod::from)
            .collect();

        if !chain_link.evolves_to.is_empty() {
            let evolves_to = chain_link
                .evolves_to
                .into_iter()
                .map(EvolutionStep::from)
                .collect();

            EvolutionStep::new(chain_link.species.name, evolution_methods, evolves_to)
        } else {
            EvolutionStep::new(chain_link.species.name, evolution_methods, vec![])
        }
    }
}

impl From<RustemonEvoMethod> for EvolutionMethod {
    fn from(evolution: RustemonEvoMethod) -> Self {
        let mut method = EvolutionMethod::new(evolution.trigger.name);
        if let Some(item) = evolution.item {
            method = method.item(item.name);
        }
        if let Some(gender) = evolution.gender {
            method = method.gender(gender);
        }
        if let Some(held_item) = evolution.held_item {
            method = method.held_item(held_item.name);
        }
        if let Some(known_move) = evolution.known_move {
            method = method.known_move(known_move.name);
        }
        if let Some(known_move_type) = evolution.known_move_type {
            method = method.known_move_type(known_move_type.name);
        }
        if let Some(location) = evolution.location {
            method = method.location(location.name);
        }
        if let Some(min_level) = evolution.min_level {
            method = method.min_level(min_level);
        }
        if let Some(min_happiness) = evolution.min_happiness {
            method = method.min_happiness(min_happiness);
        }
        if let Some(min_beauty) = evolution.min_beauty {
            method = method.min_beauty(min_beauty);
        }
        if let Some(min_affection) = evolution.min_affection {
            method = method.min_affection(min_affection);
        }
        if let Some(party_species) = evolution.party_species {
            method = method.party_species(party_species.name);
        }
        if let Some(party_type) = evolution.party_type {
            method = method.party_type(party_type.name);
        }
        if let Some(relative_physical_stats) = evolution.relative_physical_stats {
            method = method.relative_physical_stats(relative_physical_stats);
        }
        if let Some(trade_species) = evolution.trade_species {
            method = method.trade_species(trade_species.name);
        }
        if evolution.needs_overworld_rain {
            method = method.needs_overworld_rain(true);
        }
        if evolution.turn_upside_down {
            method = method.turn_upside_down(true);
        }
        if !evolution.time_of_day.is_empty() {
            method = method.time_of_day(evolution.time_of_day);
        }

        method
    }
}

impl From<Vec<RustemonStat>> for Stats {
    fn from(stats_vec: Vec<RustemonStat>) -> Self {
        let mut stats = Stats::default();

        for RustemonStat {
            stat, base_stat, ..
        } in stats_vec
        {
            match stat.name.as_str() {
                "hp" => stats.hp = base_stat,
                "attack" => stats.attack = base_stat,
                "defense" => stats.defense = base_stat,
                "special-attack" => stats.special_attack = base_stat,
                "special-defense" => stats.special_defense = base_stat,
                "speed" => stats.speed = base_stat,
                _ => (),
            }
        }

        stats
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockPast {
        gen: u8,
        value: u8,
    }

    impl Past<u8> for MockPast {
        fn generation(&self, _resource: &impl GetGeneration) -> u8 {
            self.gen
        }

        fn value(&self) -> u8 {
            self.value
        }
    }

    struct MockResource;
    impl GetGeneration for MockResource {
        fn get_gen(&self, _game: &str) -> u8 {
            0
        }
        fn get_gen_from_url(&self, _url: &str) -> u8 {
            0
        }
    }

    #[test]
    fn match_past_() {
        let mock_resource = MockResource;
        let mock_pasts = vec![
            MockPast { gen: 5, value: 5 },
            MockPast { gen: 3, value: 3 },
            MockPast { gen: 6, value: 6 },
        ];

        assert_eq!(match_past(2, &mock_pasts, &mock_resource), Some(3));
        assert_eq!(match_past(4, &mock_pasts, &mock_resource), Some(5));
        assert_eq!(match_past(6, &mock_pasts, &mock_resource), Some(6));
        assert_eq!(match_past(7, &mock_pasts, &mock_resource), None);
    }
}
