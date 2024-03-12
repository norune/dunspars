use crate::resource::GameResource;
use regex::Regex;
use std::sync::OnceLock;

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
