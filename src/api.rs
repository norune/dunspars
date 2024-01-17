use std::error::Error;
use std::collections::HashMap;

use rustemon::client::RustemonClient;
use rustemon::error::Error as RustemonError;
use rustemon::pokemon::{pokemon as rustemon_pokemon, type_ as rustemon_type};

pub struct ApiWrapper {
    client: RustemonClient
}

impl Default for ApiWrapper {
    fn default() -> ApiWrapper {
        ApiWrapper {
            client: RustemonClient::default()
        }
    }
}
impl ApiWrapper {
    pub async fn get_pokemon(&self, pokemon: &str) -> Result<GetPokemonResult, RustemonError> {
        let pokemon = rustemon_pokemon::get_by_name(&pokemon, &self.client).await?;
        let name = pokemon.name;

        let mut types = pokemon.types.iter();
        let primary_type = types.find(|t| t.slot == 1).unwrap().type_.name.clone();
        let secondary_type = match types.find(|t| t.slot == 2) {
            Some(t) => Some(t.type_.name.clone()),
            None => None
        };

        Ok(GetPokemonResult{
            name,
            primary_type, 
            secondary_type
        })
    }

    pub async fn get_type(&self, type_str: &str) -> Result<GetTypeResult, Box<dyn Error>> {
        let type_ = rustemon_type::get_by_name(type_str, &self.client).await?;
        let mut offensive_chart = HashMap::new();
        let mut defensive_chart = HashMap::new();

        type_.damage_relations.no_damage_to.iter().for_each(|t| { offensive_chart.insert(t.name.to_string(), 0.0); });
        type_.damage_relations.half_damage_to.iter().for_each(|t| { offensive_chart.insert(t.name.to_string(), 0.5); });
        type_.damage_relations.double_damage_to.iter().for_each(|t| { offensive_chart.insert(t.name.to_string(), 2.0); });

        type_.damage_relations.no_damage_from.iter().for_each(|t| { defensive_chart.insert(t.name.to_string(), 0.0); });
        type_.damage_relations.half_damage_from.iter().for_each(|t| { defensive_chart.insert(t.name.to_string(), 0.5); });
        type_.damage_relations.double_damage_from.iter().for_each(|t| { defensive_chart.insert(t.name.to_string(), 2.0); });

        Ok(GetTypeResult {
            offensive_chart,
            defensive_chart
        })
    }
}

pub struct GetPokemonResult {
    pub name: String,
    pub primary_type: String,
    pub secondary_type: Option<String>
}

pub struct GetTypeResult {
    pub offensive_chart: HashMap<String, f32>,
    pub defensive_chart: HashMap<String, f32>
}
