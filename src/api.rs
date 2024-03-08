mod convert;
pub mod once;
pub mod utils;

use crate::models::resource::{ChangeMoveValueRow, FromGen, GameRow, MoveRow};
use crate::models::{
    Ability, DefenseTypeChart, EvolutionStep, Move, OffenseTypeChart, PokemonData, PokemonGroup,
    Stats, Type, TypeChart,
};
use crate::resource::{GameResource, GetGeneration};
use convert::FromChange;
use once::{api_client, cache_manager, game_resource};
use utils::{get_all_games, get_all_moves};

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
use rustemon::model::moves::Move as RustemonMove;
use rustemon::model::pokemon::{
    Ability as RustemonAbility, Pokemon as RustemonPokemon,
    PokemonAbility as RustemonPokemonAbility, PokemonMove as RustemonPokemonMove,
    PokemonSpecies as RustemonSpecies, PokemonType as RustemonTypeSlot,
    PokemonTypePast as RustemonPastPokemonType, Type as RustemonType,
};
use rustemon::model::resource::VerboseEffect as RustemonVerboseEffect;

pub async fn get_all_game_rows() -> Result<Vec<GameRow>> {
    let game_names = get_all_games(api_client()).await?;
    let game_data_futures: FuturesOrdered<_> = game_names
        .iter()
        .map(|g| rustemon_version::get_by_name(g, api_client()))
        .collect();
    let game_results: Vec<_> = game_data_futures.collect().await;
    let mut game_data = vec![];

    for version_group in game_results {
        let game = GameRow::from(version_group?);
        game_data.push(game);
    }

    Ok(game_data)
}

pub async fn get_all_move_rows(db: &Connection) -> Result<(Vec<MoveRow>, Vec<ChangeMoveValueRow>)> {
    let move_names = get_all_moves(api_client()).await?;
    println!("retrieving moves");
    let move_futures: FuturesOrdered<_> = move_names
        .iter()
        .map(|g| rustemon_moves::get_by_name(g, api_client()))
        .collect();
    let move_results: Vec<_> = move_futures.collect().await;
    let mut move_data = vec![];
    let mut change_move_data = vec![];
    println!("moves retrieved");

    for move_ in move_results {
        let move_ = move_?;
        for past_value in move_.past_values.iter() {
            let change_move = ChangeMoveValueRow::from_change(past_value, move_.id, db);
            change_move_data.push(change_move);
        }

        let move_ = MoveRow::from(move_);
        move_data.push(move_);
    }

    Ok((move_data, change_move_data))
}

pub async fn get_move_db(move_name: &str, current_gen: u8, db: &Connection) -> Result<Move> {
    let move_row = MoveRow::from_name(move_name, db)?;
    Move::from_gen(move_row, current_gen, db)
}

pub async fn get_type(type_name: &str, current_gen: u8) -> Result<Type> {
    rustemon_type(type_name, current_gen, api_client(), game_resource()).await
}
async fn rustemon_type(
    type_name: &str,
    current_gen: u8,
    client: &RustemonClient,
    game_resource: &GameResource,
) -> Result<Type> {
    let RustemonType {
        name,
        damage_relations,
        past_damage_relations,
        generation,
        ..
    } = rustemon_type::get_by_name(type_name, client).await?;

    if current_gen < game_resource.get_gen_from_url(&generation.url) {
        bail!(format!(
            "Type '{type_name}' is not present in generation {current_gen}"
        ))
    }

    let relations = utils::match_past(current_gen, &past_damage_relations, game_resource)
        .unwrap_or(damage_relations);

    let mut offense_chart = OffenseTypeChart::from(&relations);
    offense_chart.set_label(type_name);
    let mut defense_chart = DefenseTypeChart::from(&relations);
    defense_chart.set_label(type_name);

    Ok(Type {
        name,
        offense_chart,
        defense_chart,
        generation: current_gen,
    })
}

pub async fn get_move(move_name: &str, current_gen: u8) -> Result<Move> {
    rustemon_move(move_name, current_gen, api_client(), game_resource()).await
}
async fn rustemon_move(
    move_name: &str,
    current_gen: u8,
    client: &RustemonClient,
    game_resource: &GameResource,
) -> Result<Move> {
    let RustemonMove {
        name,
        mut accuracy,
        mut power,
        mut pp,
        damage_class,
        mut type_,
        mut effect_chance,
        effect_entries,
        effect_changes,
        past_values,
        generation,
        ..
    } = rustemon_moves::get_by_name(move_name, client).await?;

    if current_gen < game_resource.get_gen_from_url(&generation.url) {
        bail!(format!(
            "Move '{move_name}' is not present in generation {current_gen}"
        ))
    }

    let RustemonVerboseEffect { mut effect, .. } = effect_entries
        .into_iter()
        .find(|e| e.language.name == "en")
        .unwrap_or_default();

    if let Some(past_stats) = utils::match_past(current_gen, &past_values, game_resource) {
        accuracy = past_stats.accuracy.or(accuracy);
        power = past_stats.power.or(power);
        pp = past_stats.pp.or(pp);
        effect_chance = past_stats.effect_chance.or(effect_chance);

        if let Some(t) = past_stats.type_ {
            type_ = t;
        }

        if let Some(entry) = past_stats
            .effect_entries
            .into_iter()
            .find(|e| e.language.name == "en")
        {
            effect = entry.effect;
        }
    }

    if let Some(past_effects) = utils::match_past(current_gen, &effect_changes, game_resource) {
        if let Some(past_effect) = past_effects.into_iter().find(|e| e.language.name == "en") {
            effect += format!("\n\nGeneration {current_gen}: {}", past_effect.effect).as_str();
        }
    }

    Ok(Move {
        name,
        accuracy,
        power,
        pp,
        damage_class: damage_class.name,
        type_: type_.name,
        effect_chance,
        effect,
        generation: current_gen,
    })
}

pub async fn get_ability(ability_name: &str, current_gen: u8) -> Result<Ability> {
    rustemon_ability(ability_name, current_gen, api_client(), game_resource()).await
}
async fn rustemon_ability(
    ability_name: &str,
    current_gen: u8,
    client: &RustemonClient,
    game_resource: &GameResource,
) -> Result<Ability> {
    let RustemonAbility {
        name,
        effect_entries,
        effect_changes,
        generation,
        ..
    } = rustemon_ability::get_by_name(ability_name, client).await?;

    if current_gen < game_resource.get_gen_from_url(&generation.url) {
        bail!(format!(
            "Ability '{ability_name}' is not present in generation {current_gen}"
        ))
    }

    let RustemonVerboseEffect {
        mut effect,
        short_effect,
        ..
    } = effect_entries
        .into_iter()
        .find(|e| e.language.name == "en")
        .unwrap_or_default();

    if let Some(past_effects) = utils::match_past(current_gen, &effect_changes, game_resource) {
        if let Some(past_effect) = past_effects.into_iter().find(|e| e.language.name == "en") {
            effect += format!("\n\nGeneration {current_gen}: {}", past_effect.effect).as_str();
        }
    }

    Ok(Ability {
        name,
        effect,
        short_effect,
        generation: current_gen,
    })
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

    #[tokio::test]
    async fn get_type_test() {
        // Fairy was not introduced until gen 6
        get_type("fairy", 5).await.unwrap_err();
        get_type("fairy", 6).await.unwrap();

        // Bug gen 1 2x against poison
        let bug_gen_1 = get_type("bug", 1).await.unwrap();
        assert_eq!(2.0, bug_gen_1.offense_chart.get_multiplier("poison"));
        assert_eq!(1.0, bug_gen_1.offense_chart.get_multiplier("dark"));

        // Bug gen >=2 2x against dark
        let bug_gen_2 = get_type("bug", 2).await.unwrap();
        assert_eq!(0.5, bug_gen_2.offense_chart.get_multiplier("poison"));
        assert_eq!(2.0, bug_gen_2.offense_chart.get_multiplier("dark"));
    }

    #[tokio::test]
    async fn get_move_test() {
        // Earth Power was not introduced until gen 4
        get_move("earth-power", 3).await.unwrap_err();
        get_move("earth-power", 4).await.unwrap();

        // Tackle gen 1-4 power: 35 accuracy: 95
        let tackle_gen_4 = get_move("tackle", 4).await.unwrap();
        assert_eq!(35, tackle_gen_4.power.unwrap());
        assert_eq!(95, tackle_gen_4.accuracy.unwrap());

        // Tackle gen 5-6 power: 50 accuracy: 100
        let tackle_gen_5 = get_move("tackle", 5).await.unwrap();
        assert_eq!(50, tackle_gen_5.power.unwrap());
        assert_eq!(100, tackle_gen_5.accuracy.unwrap());

        // Tackle gen >=7 power: 40 accuracy: 100
        let tackle_gen_9 = get_move("tackle", 9).await.unwrap();
        assert_eq!(40, tackle_gen_9.power.unwrap());
        assert_eq!(100, tackle_gen_9.accuracy.unwrap());
    }

    #[tokio::test]
    async fn get_ability_test() {
        // Beads of Ruin was not introduced until gen 9
        get_ability("beads-of-ruin", 8).await.unwrap_err();
        get_ability("beads-of-ruin", 9).await.unwrap();
    }

    #[tokio::test]
    async fn get_pokemon_test() {
        // Ogerpon was not inroduced until gen 9
        get_pokemon("ogerpon", "sword-shield").await.unwrap_err();
        get_pokemon("ogerpon", "the-teal-mask").await.unwrap();

        // Wailord is not present in gen 9, but is present in gen 8
        get_pokemon("wailord", "scarlet-violet").await.unwrap_err();
        get_pokemon("wailord", "sword-shield").await.unwrap();

        // Test dual type defense chart
        let golem = get_pokemon("golem", "scarlet-violet").await.unwrap();
        let golem_defense = golem.get_defense_chart().await.unwrap();
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
