use crate::data::resource::{app_directory_cache, GameResource};

use std::sync::OnceLock;

use regex::Regex;
use rustemon::client::{CACacheManager, CacheMode, RustemonClient, RustemonClientBuilder};

pub fn api_client() -> &'static RustemonClient {
    static API_CLIENT: OnceLock<RustemonClient> = OnceLock::new();

    API_CLIENT.get_or_init(|| {
        let cache_dir = app_directory_cache("rustemon");

        let cache_manager = CACacheManager { path: cache_dir };
        // This disregards cache staleness. Pokémon data is not likely to change
        // Cache should be cleared by user via program command
        let cache_mode = CacheMode::ForceCache;
        RustemonClientBuilder::default()
            .with_manager(cache_manager.clone())
            .with_mode(cache_mode)
            .try_build()
            .unwrap()
    })
}

pub fn game_resource() -> &'static GameResource {
    static GAME_RESOURCE: OnceLock<GameResource> = OnceLock::new();

    GAME_RESOURCE.get_or_init(|| GameResource::try_new().unwrap())
}

pub fn gen_url_regex() -> &'static Regex {
    static GEN_URL_REGEX: OnceLock<Regex> = OnceLock::new();

    GEN_URL_REGEX.get_or_init(|| {
        // PokéAPI keeps generation names in Roman numerals.
        // Might be quicker to just take it from resource urls via regex instead.
        // Regex compilation is expensive, so we're compiling it just once here.
        Regex::new(r"generation/(?P<gen>\d+)/?$").unwrap()
    })
}
