mod convert;
pub mod once;
pub mod utils;

use crate::models::resource::{
    AbilityRow, GameRow, MoveChangeRow, MoveRow, TypeChangeRow, TypeRow,
};
use crate::models::{EvolutionStep, PokemonData, PokemonGroup, Stats};
use crate::resource::{GameResource, GetGeneration};
use convert::FromChange;
use once::{api_client, cache_manager, game_resource};
use utils::{get_all_abilities, get_all_games, get_all_moves, get_all_types};

use std::collections::HashMap;

use anyhow::{anyhow, bail, Result};
use futures::stream::FuturesOrdered;
use futures::StreamExt;
use rusqlite::Connection;
use rustemon::client::RustemonClient;
use rustemon::games::version_group as rustemon_version;
use rustemon::moves::move_ as rustemon_moves;
use rustemon::pokemon::pokemon_species as rustemon_species;
use rustemon::pokemon::{
    ability as rustemon_ability, pokemon as rustemon_pokemon, type_ as rustemon_type,
};
use rustemon::Follow;

use rustemon::model::evolution::EvolutionChain as RustemonEvoRoot;
use rustemon::model::pokemon::{
    Pokemon as RustemonPokemon, PokemonAbility as RustemonPokemonAbility,
    PokemonMove as RustemonPokemonMove, PokemonSpecies as RustemonSpecies,
    PokemonType as RustemonTypeSlot, PokemonTypePast as RustemonPastPokemonType,
};

pub async fn get_all_game_rows(client: &RustemonClient) -> Result<Vec<GameRow>> {
    let game_names = get_all_games(client).await?;
    let game_data_futures: FuturesOrdered<_> = game_names
        .iter()
        .map(|g| rustemon_version::get_by_name(g, client))
        .collect();
    let game_results: Vec<_> = game_data_futures.collect().await;
    let mut game_data = vec![];

    for version_group in game_results {
        let game = GameRow::from(version_group?);
        game_data.push(game);
    }

    Ok(game_data)
}

pub async fn get_all_move_rows(
    client: &RustemonClient,
    db: &Connection,
) -> Result<(Vec<MoveRow>, Vec<MoveChangeRow>)> {
    let move_names = get_all_moves(client).await?;
    let move_futures: FuturesOrdered<_> = move_names
        .iter()
        .map(|g| rustemon_moves::get_by_name(g, client))
        .collect();
    let move_results: Vec<_> = move_futures.collect().await;
    let mut move_data = vec![];
    let mut change_move_data = vec![];

    for move_ in move_results {
        let move_ = move_?;
        for past_value in move_.past_values.iter() {
            let change_move = MoveChangeRow::from_change(past_value, move_.id, db);
            change_move_data.push(change_move);
        }

        let move_ = MoveRow::from(move_);
        move_data.push(move_);
    }

    Ok((move_data, change_move_data))
}

pub async fn get_all_type_rows(
    client: &RustemonClient,
    db: &Connection,
) -> Result<(Vec<TypeRow>, Vec<TypeChangeRow>)> {
    let type_names = get_all_types(client).await?;
    let type_futures: FuturesOrdered<_> = type_names
        .iter()
        .map(|g| rustemon_type::get_by_name(g, client))
        .collect();
    let type_results: Vec<_> = type_futures.collect().await;
    let mut type_data = vec![];
    let mut change_type_data = vec![];

    for type_ in type_results {
        let type_ = type_?;
        for past_type in type_.past_damage_relations.iter() {
            let change_move = TypeChangeRow::from_change(past_type, type_.id, db);
            change_type_data.push(change_move);
        }

        let move_ = TypeRow::from(type_);
        type_data.push(move_);
    }

    Ok((type_data, change_type_data))
}

pub async fn get_all_ability_rows(client: &RustemonClient) -> Result<Vec<AbilityRow>> {
    let ability_names = get_all_abilities(client).await?;
    let ability_futures: FuturesOrdered<_> = ability_names
        .iter()
        .map(|g| rustemon_ability::get_by_name(g, client))
        .collect();
    let ability_results: Vec<_> = ability_futures.collect().await;
    let mut ability_data = vec![];

    for ability in ability_results {
        let ability = AbilityRow::from(ability?);
        ability_data.push(ability);
    }

    Ok(ability_data)
}

pub async fn get_evolution(species: &str) -> Result<EvolutionStep> {
    rustemon_evolution(species, api_client()).await
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
    rustemon_pokemon(pokemon_name, game, api_client(), game_resource()).await
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

pub async fn clear_cache() -> Result<()> {
    match cache_manager().clear().await {
        std::result::Result::Ok(_) => Ok(()),
        std::result::Result::Err(e) => Err(anyhow!(e)),
    }
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
