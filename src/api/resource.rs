use std::collections::HashMap;

use anyhow::{anyhow, bail, Result};
use futures::stream::FuturesOrdered;
use futures::StreamExt;
use regex::Regex;

use rustemon::client::RustemonClient;
use rustemon::games::version_group as rustemon_version;

use super::utils;
use crate::pokemon::Game;

pub enum ResourceResult {
    Valid,
    Invalid(Vec<String>),
}

#[allow(async_fn_in_trait)]
pub trait Resource: Sized {
    fn get_matches(&self, value: &str) -> Vec<String> {
        self.resource()
            .iter()
            .filter_map(|r| {
                let close_enough = if !r.is_empty() && !value.is_empty() {
                    let first_r = r.chars().next().unwrap();
                    let first_value = value.chars().next().unwrap();

                    // Only perform spellcheck on first character match; potentially expensive
                    first_r == first_value && strsim::levenshtein(r, value) < 4
                } else {
                    false
                };

                if r.contains(value) || close_enough {
                    Some(r.clone())
                } else {
                    None
                }
            })
            .collect::<Vec<String>>()
    }

    fn check(&self, value: &str) -> ResourceResult {
        let matches = self.get_matches(value);
        if matches.iter().any(|m| *m == value) {
            ResourceResult::Valid
        } else {
            ResourceResult::Invalid(matches)
        }
    }

    fn validate(&self, value: &str) -> Result<String> {
        let value = value.to_lowercase();
        match self.check(&value) {
            ResourceResult::Valid => Ok(value),
            ResourceResult::Invalid(matches) => bail!(Self::invalid_message(&value, &matches)),
        }
    }

    fn invalid_message(value: &str, matches: &[String]) -> String {
        let resource_name = Self::label();
        let mut message = format!("{resource_name} '{value}' not found.");

        if matches.len() > 20 {
            message += " Potential matches found; too many to display.";
        } else if !matches.is_empty() {
            message += &format!(" Potential matches: {}.", matches.join(" "));
        }

        message
    }

    async fn try_new(api: &RustemonClient) -> Result<Self>;
    fn resource(&self) -> Vec<String>;
    fn label() -> &'static str;
}

#[derive(Debug)]
pub struct PokemonResource {
    resource: Vec<String>,
}
impl Resource for PokemonResource {
    async fn try_new(client: &RustemonClient) -> Result<Self> {
        let resource = utils::get_all_pokemon(client).await?;
        Ok(Self { resource })
    }

    fn resource(&self) -> Vec<String> {
        self.resource.clone()
    }

    fn label() -> &'static str {
        "Pokémon"
    }
}

#[derive(Debug)]
pub struct TypeResource {
    resource: Vec<String>,
}
impl Resource for TypeResource {
    async fn try_new(client: &RustemonClient) -> Result<Self> {
        let resource = utils::get_all_types(client).await?;
        Ok(Self { resource })
    }

    fn resource(&self) -> Vec<String> {
        self.resource.clone()
    }

    fn label() -> &'static str {
        "Type"
    }
}

#[derive(Debug)]
pub struct MoveResource {
    resource: Vec<String>,
}
impl Resource for MoveResource {
    async fn try_new(client: &RustemonClient) -> Result<Self> {
        let resource = utils::get_all_moves(client).await?;
        Ok(Self { resource })
    }

    fn resource(&self) -> Vec<String> {
        self.resource.clone()
    }

    fn label() -> &'static str {
        "Move"
    }
}

#[derive(Debug)]
pub struct AbilityResource {
    resource: Vec<String>,
}
impl Resource for AbilityResource {
    async fn try_new(client: &RustemonClient) -> Result<Self> {
        let resource = utils::get_all_abilities(client).await?;
        Ok(Self { resource })
    }

    fn resource(&self) -> Vec<String> {
        self.resource.clone()
    }

    fn label() -> &'static str {
        "Ability"
    }
}

#[derive(Debug)]
pub struct GameResource {
    resource: HashMap<String, Game>,
    gen_url_regex: Regex,
}
impl Resource for GameResource {
    async fn try_new(client: &RustemonClient) -> Result<Self> {
        // PokéAPI keeps generation names in Roman numerals.
        // Might be quicker to just take it from resource urls via regex instead.
        // Regex compilation is expensive, so we're compiling it just once here.
        let gen_url_regex = Regex::new(r"generation/(?P<gen>\d+)/?$").unwrap();

        let mut resource = HashMap::new();
        let game_names = utils::get_all_games(client).await?;
        let game_data_futures: FuturesOrdered<_> = game_names
            .iter()
            .map(|g| rustemon_version::get_by_name(g, client))
            .collect();
        let game_data: Vec<_> = game_data_futures.collect().await;

        for (i, game) in game_data.into_iter().enumerate() {
            let game = game?;
            let generation = capture_gen_url(&game.generation.url, &gen_url_regex).unwrap();
            resource.insert(game.name.clone(), Game::new(game.name, i as u8, generation));
        }

        Ok(Self {
            resource,
            gen_url_regex,
        })
    }

    fn resource(&self) -> Vec<String> {
        let mut games = self.resource.iter().map(|r| r.1).collect::<Vec<&Game>>();
        games.sort_by_key(|g| g.order);

        games
            .iter()
            .map(|g| g.name.clone())
            .collect::<Vec<String>>()
    }

    fn label() -> &'static str {
        "Game"
    }
}
impl GameResource {
    pub fn get_gen(&self, game: &str) -> u8 {
        self.resource.get(game).unwrap().generation
    }

    pub fn get_gen_from_url(&self, url: &str) -> u8 {
        capture_gen_url(url, &self.gen_url_regex).unwrap()
    }
}

fn capture_gen_url(url: &str, gen_url_regex: &Regex) -> Result<u8> {
    if let Some(caps) = gen_url_regex.captures(url) {
        Ok(caps["gen"].parse::<u8>()?)
    } else {
        Err(anyhow!("Generation not found in resource url"))
    }
}
