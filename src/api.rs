pub mod convert;
pub mod once;
pub mod utils;

use crate::models::{EvolutionStep, PokemonData, PokemonGroup, Stats};
use crate::resource::{GameResource, GetGeneration};
use once::game_resource;

use std::collections::HashMap;

use anyhow::{bail, Result};
use rustemon::client::{CacheMode, RustemonClient, RustemonClientBuilder};
use rustemon::pokemon::pokemon as rustemon_pokemon;
use rustemon::pokemon::pokemon_species as rustemon_species;
use rustemon::Follow;

use rustemon::model::evolution::EvolutionChain as RustemonEvoRoot;
use rustemon::model::pokemon::{
    Pokemon as RustemonPokemon, PokemonAbility as RustemonPokemonAbility,
    PokemonMove as RustemonPokemonMove, PokemonSpecies as RustemonSpecies,
    PokemonType as RustemonTypeSlot, PokemonTypePast as RustemonPastPokemonType,
};

pub fn api_client() -> RustemonClient {
    // This disregards cache staleness. Pokémon data is not likely to change
    // Cache should be cleared by user via program command
    let cache_mode = CacheMode::ForceCache;
    RustemonClientBuilder::default()
        .with_mode(cache_mode)
        .try_build()
        .unwrap()
}

pub async fn get_evolution(species: &str) -> Result<EvolutionStep> {
    rustemon_evolution(species, &api_client()).await
}
async fn rustemon_evolution(species: &str, client: &RustemonClient) -> Result<EvolutionStep> {
    let RustemonEvoRoot { chain, .. } = rustemon_species::get_by_name(species, client)
        .await?
        .evolution_chain
        .unwrap()
        .follow(client)
        .await?;
    let evolution_step = EvolutionStep::from(chain);

    Ok(evolution_step)
}

pub async fn get_pokemon(pokemon_name: &str, game: &str) -> Result<PokemonData> {
    rustemon_pokemon(pokemon_name, game, &api_client(), game_resource()).await
}
pub async fn rustemon_pokemon(
    pokemon_name: &str,
    game: &str,
    client: &RustemonClient,
    game_resource: &GameResource,
) -> Result<PokemonData> {
    let RustemonPokemon {
        name,
        types,
        past_types,
        moves,
        stats,
        abilities,
        species,
        ..
    } = rustemon_pokemon::get_by_name(pokemon_name, client).await?;

    let current_generation = game_resource.get_gen(game);
    let learn_moves = get_pokemon_moves(moves, current_generation, game_resource);
    // PokéAPI doesn't seem to supply a field that denotes when a Pokémon was introduced.
    // So the next best thing is to check if they have any moves in the specified generation.
    if learn_moves.is_empty() {
        bail!(format!(
            "Pokémon '{pokemon_name}' is not present in generation {current_generation}"
        ))
    }

    let (primary_type, secondary_type) =
        get_pokemon_type(types, past_types, current_generation, game_resource);
    let abilities = get_pokemon_abilities(abilities);

    let group = get_pokemon_group(&species.name, client).await?;

    Ok(PokemonData {
        name,
        primary_type,
        secondary_type,
        learn_moves,
        abilities,
        species: species.name,
        group,
        stats: Stats::from(stats),
        game: game.to_string(),
        generation: current_generation,
    })
}

fn get_pokemon_type(
    types: Vec<RustemonTypeSlot>,
    past_types: Vec<RustemonPastPokemonType>,
    generation: u8,
    game_resource: &GameResource,
) -> (String, Option<String>) {
    let pokemon_types = utils::match_past(generation, &past_types, game_resource).unwrap_or(types);

    let primary_type = pokemon_types
        .iter()
        .find(|t| t.slot == 1)
        .unwrap()
        .type_
        .name
        .clone();
    let secondary_type = pokemon_types
        .iter()
        .find(|t| t.slot == 2)
        .map(|t| t.type_.name.clone());

    (primary_type, secondary_type)
}

fn get_pokemon_moves(
    moves: Vec<RustemonPokemonMove>,
    generation: u8,
    game_resource: &GameResource,
) -> HashMap<String, (String, i64)> {
    let mut learn_moves = HashMap::new();
    for move_ in moves {
        let learnable_move = move_.version_group_details.iter().find(|vg| {
            let vg_gen = game_resource.get_gen(&vg.version_group.name);
            vg_gen == generation
        });

        if let Some(learn_move) = learnable_move {
            learn_moves.insert(
                move_.move_.name.clone(),
                (
                    learn_move.move_learn_method.name.clone(),
                    learn_move.level_learned_at,
                ),
            );
        }
    }
    learn_moves
}

fn get_pokemon_abilities(abilities: Vec<RustemonPokemonAbility>) -> Vec<(String, bool)> {
    abilities
        .iter()
        .map(|a| (a.ability.name.clone(), a.is_hidden))
        .collect::<Vec<_>>()
}

async fn get_pokemon_group(species: &str, client: &RustemonClient) -> Result<PokemonGroup> {
    let RustemonSpecies {
        is_legendary,
        is_mythical,
        ..
    } = rustemon_species::get_by_name(species, client).await?;

    if is_mythical {
        return Ok(PokemonGroup::Mythical);
    }

    if is_legendary {
        return Ok(PokemonGroup::Legendary);
    }

    Ok(PokemonGroup::Regular)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::TypeChart;
    use crate::resource::DatabaseFile;

    #[tokio::test]
    async fn get_pokemon_test() {
        let DatabaseFile { ref db, .. } = DatabaseFile::try_new(false).unwrap();

        // Ogerpon was not inroduced until gen 9
        get_pokemon("ogerpon", "sword-shield").await.unwrap_err();
        get_pokemon("ogerpon", "the-teal-mask").await.unwrap();

        // Wailord is not present in gen 9, but is present in gen 8
        get_pokemon("wailord", "scarlet-violet").await.unwrap_err();
        get_pokemon("wailord", "sword-shield").await.unwrap();

        // Test dual type defense chart
        let golem = get_pokemon("golem", "scarlet-violet").await.unwrap();
        let golem_defense = golem.get_defense_chart(db).unwrap();
        assert_eq!(4.0, golem_defense.get_multiplier("water"));
        assert_eq!(2.0, golem_defense.get_multiplier("fighting"));
        assert_eq!(1.0, golem_defense.get_multiplier("psychic"));
        assert_eq!(0.5, golem_defense.get_multiplier("flying"));
        assert_eq!(0.25, golem_defense.get_multiplier("poison"));
        assert_eq!(0.0, golem_defense.get_multiplier("electric"));

        // Clefairy was Normal type until gen 6
        let clefairy_gen_5 = get_pokemon("clefairy", "black-white").await.unwrap();
        assert_eq!("normal", clefairy_gen_5.primary_type);
        let clefairy_gen_6 = get_pokemon("clefairy", "x-y").await.unwrap();
        assert_eq!("fairy", clefairy_gen_6.primary_type);
    }

    #[tokio::test]
    async fn get_evolution_test() {
        let cascoon = get_evolution("cascoon").await.unwrap();
        insta::assert_yaml_snapshot!(cascoon);

        let applin = get_evolution("applin").await.unwrap();
        insta::assert_yaml_snapshot!(applin);

        let politoed = get_evolution("politoed").await.unwrap();
        insta::assert_yaml_snapshot!(politoed);
    }
}
