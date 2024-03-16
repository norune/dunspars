mod convert;

use crate::models::resource::{
    AbilityRow, EvolutionRow, GameRow, InsertRow, MoveChangeRow, MoveRow, MoveRowGroup,
    PokemonAbilityRow, PokemonMoveRow, PokemonRow, PokemonRowGroup, PokemonTypeChangeRow,
    SelectRow, SpeciesRow, TypeChangeRow, TypeRow, TypeRowGroup,
};
use crate::models::EvolutionStep;
use convert::{capture_url_id, FromChange};

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
pub trait FetchIdentifiers {
    type Identifier;

    async fn fetch_all_identifiers(client: &RustemonClient) -> Result<Vec<Self::Identifier>>;
}

#[allow(async_fn_in_trait)]
pub trait FetchEntries: FetchIdentifiers {
    type Entry;

    async fn fetch_all_entries(
        identifiers: Vec<Self::Identifier>,
        client: &RustemonClient,
    ) -> Result<Vec<Self::Entry>> {
        // Entry retrieval needs to be done in chunks because sending too many TCP requests
        // concurrently can cause "tcp open error: Too many open files (os error 24)"
        let chunked_identifiers = identifiers.chunks(100);
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
    async fn fetch_entry(
        identifier: &Self::Identifier,
        client: &RustemonClient,
    ) -> Result<Self::Entry>;
}

pub trait ConvertEntries: FetchEntries {
    type Row: InsertRow;

    fn convert_to_rows(entries: Vec<Self::Entry>, db: &Connection) -> Vec<Self::Row>;
}

#[allow(async_fn_in_trait)]
pub trait FetchResource: FetchIdentifiers + FetchEntries + ConvertEntries {
    async fn fetch_resource(client: &RustemonClient, db: &Connection) -> Result<Vec<Self::Row>> {
        let names = Self::fetch_all_identifiers(client).await?;
        let entries = Self::fetch_all_entries(names, client).await?;
        Ok(Self::convert_to_rows(entries, db))
    }
}

pub struct GameFetcher;
impl FetchIdentifiers for GameFetcher {
    type Identifier = String;

    async fn fetch_all_identifiers(client: &RustemonClient) -> Result<Vec<String>> {
        Ok(rustemon_version::get_all_entries(client)
            .await?
            .into_iter()
            .map(|g| g.name)
            .collect::<Vec<String>>())
    }
}
impl FetchEntries for GameFetcher {
    type Entry = VersionGroup;

    async fn fetch_entry(identifier: &String, client: &RustemonClient) -> Result<VersionGroup> {
        Ok(rustemon_version::get_by_name(identifier, client).await?)
    }
}
impl ConvertEntries for GameFetcher {
    type Row = GameRow;

    fn convert_to_rows(entries: Vec<VersionGroup>, _db: &Connection) -> Vec<GameRow> {
        entries
            .into_iter()
            .map(GameRow::from)
            .collect::<Vec<GameRow>>()
    }
}
impl FetchResource for GameFetcher {}

pub struct MoveFetcher;
impl FetchIdentifiers for MoveFetcher {
    type Identifier = String;

    async fn fetch_all_identifiers(client: &RustemonClient) -> Result<Vec<String>> {
        Ok(rustemon_move::get_all_entries(client)
            .await?
            .into_iter()
            .map(|g| g.name)
            .collect::<Vec<String>>())
    }
}
impl FetchEntries for MoveFetcher {
    type Entry = Move;

    async fn fetch_entry(identifier: &String, client: &RustemonClient) -> Result<Move> {
        Ok(rustemon_move::get_by_name(identifier, client).await?)
    }
}
impl ConvertEntries for MoveFetcher {
    type Row = MoveRowGroup;

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
impl FetchResource for MoveFetcher {}

pub struct TypeFetcher;
impl FetchIdentifiers for TypeFetcher {
    type Identifier = String;

    async fn fetch_all_identifiers(client: &RustemonClient) -> Result<Vec<String>> {
        Ok(rustemon_type::get_all_entries(client)
            .await?
            .into_iter()
            .map(|g| g.name)
            .collect::<Vec<String>>())
    }
}
impl FetchEntries for TypeFetcher {
    type Entry = Type;

    async fn fetch_entry(identifier: &String, client: &RustemonClient) -> Result<Type> {
        Ok(rustemon_type::get_by_name(identifier, client).await?)
    }
}
impl ConvertEntries for TypeFetcher {
    type Row = TypeRowGroup;

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
impl FetchResource for TypeFetcher {}

pub struct AbilityFetcher;
impl FetchIdentifiers for AbilityFetcher {
    type Identifier = String;

    async fn fetch_all_identifiers(client: &RustemonClient) -> Result<Vec<String>> {
        Ok(rustemon_ability::get_all_entries(client)
            .await?
            .into_iter()
            .map(|g| g.name)
            .collect::<Vec<String>>())
    }
}
impl FetchEntries for AbilityFetcher {
    type Entry = Ability;

    async fn fetch_entry(identifier: &String, client: &RustemonClient) -> Result<Ability> {
        Ok(rustemon_ability::get_by_name(identifier, client).await?)
    }
}
impl ConvertEntries for AbilityFetcher {
    type Row = AbilityRow;

    fn convert_to_rows(entries: Vec<Ability>, _db: &Connection) -> Vec<AbilityRow> {
        entries
            .into_iter()
            .map(AbilityRow::from)
            .collect::<Vec<AbilityRow>>()
    }
}
impl FetchResource for AbilityFetcher {}

pub struct SpeciesFetcher;
impl FetchIdentifiers for SpeciesFetcher {
    type Identifier = String;

    async fn fetch_all_identifiers(client: &RustemonClient) -> Result<Vec<String>> {
        Ok(rustemon_species::get_all_entries(client)
            .await?
            .into_iter()
            .map(|g| g.name)
            .collect::<Vec<String>>())
    }
}
impl FetchEntries for SpeciesFetcher {
    type Entry = PokemonSpecies;

    async fn fetch_entry(identifier: &String, client: &RustemonClient) -> Result<PokemonSpecies> {
        Ok(rustemon_species::get_by_name(identifier, client).await?)
    }
}
impl ConvertEntries for SpeciesFetcher {
    type Row = SpeciesRow;

    fn convert_to_rows(entries: Vec<PokemonSpecies>, _db: &Connection) -> Vec<SpeciesRow> {
        entries
            .into_iter()
            .map(SpeciesRow::from)
            .collect::<Vec<SpeciesRow>>()
    }
}
impl FetchResource for SpeciesFetcher {}

pub struct EvolutionFetcher;
impl FetchIdentifiers for EvolutionFetcher {
    type Identifier = i64;

    async fn fetch_all_identifiers(client: &RustemonClient) -> Result<Vec<i64>> {
        // rustemon::evolution::evolution_chain::get_all_entries() is broken.
        // Retrieve them instead via species resource instead.
        let names = SpeciesFetcher::fetch_all_identifiers(client).await?;
        let species = SpeciesFetcher::fetch_all_entries(names, client).await?;
        let mut evolution_ids = HashSet::new();

        for specie in species {
            if let Some(evolution) = specie.evolution_chain {
                evolution_ids.insert(capture_url_id(&evolution.url).unwrap());
            }
        }

        Ok(evolution_ids.into_iter().collect())
    }
}
impl FetchEntries for EvolutionFetcher {
    type Entry = EvolutionChain;

    async fn fetch_entry(identifier: &i64, client: &RustemonClient) -> Result<EvolutionChain> {
        Ok(rustemon_evolution::get_by_id(*identifier, client).await?)
    }
}
impl ConvertEntries for EvolutionFetcher {
    type Row = EvolutionRow;

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
impl FetchResource for EvolutionFetcher {}

pub struct PokemonFetcher;
impl FetchIdentifiers for PokemonFetcher {
    type Identifier = String;

    async fn fetch_all_identifiers(client: &RustemonClient) -> Result<Vec<String>> {
        Ok(rustemon_pokemon::get_all_entries(client)
            .await?
            .into_iter()
            .map(|g| g.name)
            .collect::<Vec<String>>())
    }
}
impl FetchEntries for PokemonFetcher {
    type Entry = Pokemon;

    async fn fetch_entry(identifier: &String, client: &RustemonClient) -> Result<Pokemon> {
        Ok(rustemon_pokemon::get_by_name(identifier, client).await?)
    }
}
impl ConvertEntries for PokemonFetcher {
    type Row = PokemonRowGroup;

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
impl FetchResource for PokemonFetcher {}
