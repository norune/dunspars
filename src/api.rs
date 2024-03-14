mod convert;

use crate::models::resource::{
    AbilityRow, EvolutionRow, GameRow, MoveChangeRow, MoveRow, MoveRowGroup, PokemonAbilityRow,
    PokemonMoveRow, PokemonRow, PokemonRowGroup, PokemonTypeChangeRow, SelectRow, SpeciesRow,
    TypeChangeRow, TypeRow, TypeRowGroup,
};
use crate::models::EvolutionStep;
use convert::FromChange;

use std::collections::HashSet;

use anyhow::Result;
use futures::stream::FuturesUnordered;
use futures::StreamExt;
use rusqlite::Connection;

use rustemon::evolution::evolution_chain as rustemon_evolution;
use rustemon::games::version_group as rustemon_version;
use rustemon::moves::move_ as rustemon_move;
use rustemon::pokemon::ability as rustemon_ability;
use rustemon::pokemon::pokemon as rustemon_pokemon;
use rustemon::pokemon::pokemon_species as rustemon_species;
use rustemon::pokemon::type_ as rustemon_type;

use rustemon::client::{CacheMode, RustemonClient, RustemonClientBuilder};
use rustemon::model::evolution::EvolutionChain;
use rustemon::model::games::VersionGroup;
use rustemon::model::moves::Move;
use rustemon::model::pokemon::{Ability, Pokemon, PokemonSpecies, Type};

pub fn api_client() -> RustemonClient {
    RustemonClientBuilder::default()
        .with_mode(CacheMode::ForceCache)
        .try_build()
        .unwrap()
}

pub fn game_to_gen(game: &str, db: &Connection) -> u8 {
    let game = GameRow::select_by_name(game, db).unwrap();
    game.generation
}

#[allow(async_fn_in_trait)]
pub trait FetchIdentifiers<I> {
    async fn fetch_all_identifiers(client: &RustemonClient) -> Result<Vec<I>>;
}

#[allow(async_fn_in_trait)]
pub trait FetchEntries<I, E> {
    async fn fetch_all_entries(identifiers: Vec<I>, client: &RustemonClient) -> Result<Vec<E>> {
        // Entry retrieval needs to be done in chunks because sending too many TCP requests
        // concurrently can cause "tcp open error: Too many open files (os error 24)"
        let chunked_identifiers = identifiers.chunks(200);
        let mut entries = vec![];

        for chunk in chunked_identifiers {
            let entry_futures: FuturesUnordered<_> = chunk
                .iter()
                .map(|identifier| Self::fetch_entry(identifier, client))
                .collect();
            let entry_results: Vec<_> = entry_futures.collect().await;
            for entry in entry_results {
                entries.push(entry?);
            }
        }

        Ok(entries)
    }
    async fn fetch_entry(identifier: &I, client: &RustemonClient) -> Result<E>;
}

pub trait ConvertEntries<E, R> {
    fn convert_to_rows(entries: Vec<E>, db: &Connection) -> Vec<R>;
}

#[allow(async_fn_in_trait)]
pub trait FetchResource<I, E, R>:
    FetchIdentifiers<I> + FetchEntries<I, E> + ConvertEntries<E, R>
{
    async fn fetch_resource(client: &RustemonClient, db: &Connection) -> Result<Vec<R>> {
        let names = Self::fetch_all_identifiers(client).await?;
        let entries = Self::fetch_all_entries(names, client).await?;
        Ok(Self::convert_to_rows(entries, db))
    }
}

pub struct GameFetcher;
impl FetchIdentifiers<String> for GameFetcher {
    async fn fetch_all_identifiers(client: &RustemonClient) -> Result<Vec<String>> {
        Ok(rustemon_version::get_all_entries(client)
            .await?
            .into_iter()
            .map(|g| g.name)
            .collect::<Vec<String>>())
    }
}
impl FetchEntries<String, VersionGroup> for GameFetcher {
    async fn fetch_entry(identifier: &String, client: &RustemonClient) -> Result<VersionGroup> {
        Ok(rustemon_version::get_by_name(identifier, client).await?)
    }
}
impl ConvertEntries<VersionGroup, GameRow> for GameFetcher {
    fn convert_to_rows(entries: Vec<VersionGroup>, _db: &Connection) -> Vec<GameRow> {
        entries
            .into_iter()
            .map(GameRow::from)
            .collect::<Vec<GameRow>>()
    }
}
impl FetchResource<String, VersionGroup, GameRow> for GameFetcher {}

pub struct MoveFetcher;
impl FetchIdentifiers<String> for MoveFetcher {
    async fn fetch_all_identifiers(client: &RustemonClient) -> Result<Vec<String>> {
        Ok(rustemon_move::get_all_entries(client)
            .await?
            .into_iter()
            .map(|g| g.name)
            .collect::<Vec<String>>())
    }
}
impl FetchEntries<String, Move> for MoveFetcher {
    async fn fetch_entry(identifier: &String, client: &RustemonClient) -> Result<Move> {
        Ok(rustemon_move::get_by_name(identifier, client).await?)
    }
}
impl ConvertEntries<Move, MoveRowGroup> for MoveFetcher {
    fn convert_to_rows(entries: Vec<Move>, db: &Connection) -> Vec<MoveRowGroup> {
        let mut move_data = vec![];

        for move_ in entries {
            for past_value in move_.past_values.iter() {
                let change_move = MoveChangeRow::from_change(past_value, move_.id, db);
                move_data.push(MoveRowGroup::MoveChangeRow(change_move));
            }

            let move_ = MoveRow::from(move_);
            move_data.push(MoveRowGroup::MoveRow(move_));
        }

        move_data
    }
}
impl FetchResource<String, Move, MoveRowGroup> for MoveFetcher {}

pub struct TypeFetcher;
impl FetchIdentifiers<String> for TypeFetcher {
    async fn fetch_all_identifiers(client: &RustemonClient) -> Result<Vec<String>> {
        Ok(rustemon_type::get_all_entries(client)
            .await?
            .into_iter()
            .map(|g| g.name)
            .collect::<Vec<String>>())
    }
}
impl FetchEntries<String, Type> for TypeFetcher {
    async fn fetch_entry(identifier: &String, client: &RustemonClient) -> Result<Type> {
        Ok(rustemon_type::get_by_name(identifier, client).await?)
    }
}
impl ConvertEntries<Type, TypeRowGroup> for TypeFetcher {
    fn convert_to_rows(entries: Vec<Type>, db: &Connection) -> Vec<TypeRowGroup> {
        let mut type_data = vec![];
        for type_ in entries {
            for past_type in type_.past_damage_relations.iter() {
                let change_move = TypeChangeRow::from_change(past_type, type_.id, db);
                type_data.push(TypeRowGroup::TypeChangeRow(change_move));
            }

            let move_ = TypeRow::from(type_);
            type_data.push(TypeRowGroup::TypeRow(move_));
        }
        type_data
    }
}
impl FetchResource<String, Type, TypeRowGroup> for TypeFetcher {}

pub struct AbilityFetcher;
impl FetchIdentifiers<String> for AbilityFetcher {
    async fn fetch_all_identifiers(client: &RustemonClient) -> Result<Vec<String>> {
        Ok(rustemon_ability::get_all_entries(client)
            .await?
            .into_iter()
            .map(|g| g.name)
            .collect::<Vec<String>>())
    }
}
impl FetchEntries<String, Ability> for AbilityFetcher {
    async fn fetch_entry(identifier: &String, client: &RustemonClient) -> Result<Ability> {
        Ok(rustemon_ability::get_by_name(identifier, client).await?)
    }
}
impl ConvertEntries<Ability, AbilityRow> for AbilityFetcher {
    fn convert_to_rows(entries: Vec<Ability>, _db: &Connection) -> Vec<AbilityRow> {
        entries
            .into_iter()
            .map(AbilityRow::from)
            .collect::<Vec<AbilityRow>>()
    }
}
impl FetchResource<String, Ability, AbilityRow> for AbilityFetcher {}

pub struct SpeciesFetcher;
impl FetchIdentifiers<String> for SpeciesFetcher {
    async fn fetch_all_identifiers(client: &RustemonClient) -> Result<Vec<String>> {
        Ok(rustemon_species::get_all_entries(client)
            .await?
            .into_iter()
            .map(|g| g.name)
            .collect::<Vec<String>>())
    }
}
impl FetchEntries<String, PokemonSpecies> for SpeciesFetcher {
    async fn fetch_entry(identifier: &String, client: &RustemonClient) -> Result<PokemonSpecies> {
        Ok(rustemon_species::get_by_name(identifier, client).await?)
    }
}
impl ConvertEntries<PokemonSpecies, SpeciesRow> for SpeciesFetcher {
    fn convert_to_rows(entries: Vec<PokemonSpecies>, _db: &Connection) -> Vec<SpeciesRow> {
        entries
            .into_iter()
            .map(SpeciesRow::from)
            .collect::<Vec<SpeciesRow>>()
    }
}
impl FetchResource<String, PokemonSpecies, SpeciesRow> for SpeciesFetcher {}

pub struct EvolutionFetcher;
impl FetchEntries<i64, EvolutionChain> for EvolutionFetcher {
    async fn fetch_entry(identifier: &i64, client: &RustemonClient) -> Result<EvolutionChain> {
        Ok(rustemon_evolution::get_by_id(*identifier, client).await?)
    }
}
impl ConvertEntries<EvolutionChain, EvolutionRow> for EvolutionFetcher {
    fn convert_to_rows(entries: Vec<EvolutionChain>, _db: &Connection) -> Vec<EvolutionRow> {
        let mut evo_data = vec![];
        for evolution in entries {
            let evolution_step = EvolutionStep::from(evolution.chain);
            let serialized_step = serde_json::to_string(&evolution_step).unwrap();
            let evolution_row = EvolutionRow {
                id: evolution.id,
                evolution: serialized_step,
            };
            evo_data.push(evolution_row);
        }
        evo_data
    }
}
impl EvolutionFetcher {
    pub async fn fetch_resource(
        species: &[SpeciesRow],
        client: &RustemonClient,
        db: &Connection,
    ) -> Result<Vec<EvolutionRow>> {
        // rustemon::evolution::evolution_chain::get_all_entries() is broken.
        // Retrieve them instead via species table foreign keys.
        let mut evolution_ids = HashSet::new();
        species.iter().for_each(|s| {
            if let Some(evolution_id) = s.evolution_id {
                evolution_ids.insert(evolution_id);
            }
        });

        let entries =
            EvolutionFetcher::fetch_all_entries(evolution_ids.into_iter().collect(), client)
                .await?;

        Ok(Self::convert_to_rows(entries, db))
    }
}

pub struct PokemonFetcher;
impl FetchIdentifiers<String> for PokemonFetcher {
    async fn fetch_all_identifiers(client: &RustemonClient) -> Result<Vec<String>> {
        Ok(rustemon_pokemon::get_all_entries(client)
            .await?
            .into_iter()
            .map(|g| g.name)
            .collect::<Vec<String>>())
    }
}
impl FetchEntries<String, Pokemon> for PokemonFetcher {
    async fn fetch_entry(identifier: &String, client: &RustemonClient) -> Result<Pokemon> {
        Ok(rustemon_pokemon::get_by_name(identifier, client).await?)
    }
}
impl ConvertEntries<Pokemon, PokemonRowGroup> for PokemonFetcher {
    fn convert_to_rows(entries: Vec<Pokemon>, db: &Connection) -> Vec<PokemonRowGroup> {
        let mut pokemon_data = vec![];
        for pokemon in entries {
            for ability in pokemon.abilities.iter() {
                let ability_row = PokemonAbilityRow::from_change(ability, pokemon.id, db);
                pokemon_data.push(PokemonRowGroup::PokemonAbilityRow(ability_row));
            }

            for move_ in pokemon.moves.iter() {
                let move_rows = Vec::<PokemonMoveRow>::from_change(move_, pokemon.id, db);
                pokemon_data.append(
                    &mut move_rows
                        .into_iter()
                        .map(PokemonRowGroup::PokemonMoveRow)
                        .collect(),
                );
            }

            for past_type in pokemon.past_types.iter() {
                let change_row = PokemonTypeChangeRow::from_change(past_type, pokemon.id, db);
                pokemon_data.push(PokemonRowGroup::PokemonTypeChangeRow(change_row));
            }

            let pokemon_row = PokemonRow::from(pokemon);
            pokemon_data.push(PokemonRowGroup::PokemonRow(pokemon_row));
        }
        pokemon_data
    }
}
impl FetchResource<String, Pokemon, PokemonRowGroup> for PokemonFetcher {}
