use std::error::Error;

use rustemon::client::RustemonClient;
use rustemon::error::Error as RustemonError;
use rustemon::pokemon::{pokemon as rustemon_pokemon, type_ as rustemon_type};

pub struct ApiWrapper {
    client: RustemonClient
}

impl ApiWrapper {
    pub fn default() -> ApiWrapper {
        ApiWrapper {
            client: RustemonClient::default()
        }
    }

    pub async fn get_pokemon(&self, pokemon: &str) -> Result<Pokemon, RustemonError> {
        let pokemon = rustemon_pokemon::get_by_name(&pokemon, &self.client).await?;

        let mut types = pokemon.types.iter();
        let primary_type = types.find(|t| t.slot == 1).unwrap().type_.name.clone();
        let secondary_type = match types.find(|t| t.slot == 2) {
            Some(t) => Some(t.type_.name.clone()),
            None => None
        };

        Ok(Pokemon {
            name: pokemon.name.clone(),
            types: (primary_type, secondary_type)
        })
    }

    pub async fn get_type_charts(&self, primary_type: &str, secondary_type: Option<&str>) -> Result<(TypeChart, TypeChart), Box<dyn Error>> {
        let mut attack_chart = TypeChart::new();
        let mut defense_chart = TypeChart::new();
        let primary_type = rustemon_type::get_by_name(primary_type, &self.client).await?;

        primary_type.damage_relations.no_damage_to.iter().for_each(|t| attack_chart.apply(&t.name, 0.0).unwrap());
        primary_type.damage_relations.half_damage_to.iter().for_each(|t| attack_chart.apply(&t.name, 0.5).unwrap());
        primary_type.damage_relations.double_damage_to.iter().for_each(|t| attack_chart.apply(&t.name, 2.0).unwrap());

        primary_type.damage_relations.no_damage_from.iter().for_each(|t| defense_chart.apply(&t.name, 0.0).unwrap());
        primary_type.damage_relations.half_damage_from.iter().for_each(|t| defense_chart.apply(&t.name, 0.5).unwrap());
        primary_type.damage_relations.double_damage_from.iter().for_each(|t| defense_chart.apply(&t.name, 2.0).unwrap());

        if let Some(t) = secondary_type {
            let secondary_type = rustemon_type::get_by_name(t, &self.client).await?;

            secondary_type.damage_relations.no_damage_from.iter().for_each(|t| defense_chart.apply(&t.name, 0.0).unwrap());
            secondary_type.damage_relations.half_damage_from.iter().for_each(|t| defense_chart.apply(&t.name, 0.5).unwrap());
            secondary_type.damage_relations.double_damage_from.iter().for_each(|t| defense_chart.apply(&t.name, 2.0).unwrap());
        }

        Ok((attack_chart, defense_chart))
    }
}

pub struct Pokemon {
    pub name: String,
    pub types: (String, Option<String>)
}

#[derive(Debug)]
pub struct TypeChart {
    pub normal: f32,
    pub fighting: f32,
    pub flying: f32,
    pub poison: f32,
    pub ground: f32,
    pub rock: f32,
    pub bug: f32,
    pub ghost: f32,
    pub steel: f32,
    pub fire: f32,
    pub water: f32,
    pub grass: f32,
    pub electric: f32,
    pub psychic: f32,
    pub ice: f32,
    pub dragon: f32,
    pub dark: f32,
    pub fairy: f32
}

impl TypeChart {
    pub fn new() -> TypeChart {
        TypeChart {
            normal: 1.0,
            fighting: 1.0,
            flying: 1.0,
            poison: 1.0,
            ground: 1.0,
            rock: 1.0,
            bug: 1.0,
            ghost: 1.0,
            steel: 1.0,
            fire: 1.0,
            water: 1.0,
            grass: 1.0,
            electric: 1.0,
            psychic: 1.0,
            ice: 1.0,
            dragon: 1.0,
            dark: 1.0,
            fairy: 1.0
        }
    }

    pub fn apply(&mut self, type_: &str, op: f32) -> Result<(), &str> {
        match type_ {
            "normal" => self.normal *= op,
            "fighting" => self.fighting *= op,
            "flying" => self.flying *= op,
            "poison" => self.poison *= op,
            "ground" => self.ground *= op,
            "rock" => self.rock *= op,
            "bug" => self.bug *= op,
            "ghost" => self.ghost *= op,
            "steel" => self.steel *= op,
            "fire" => self.fire *= op,
            "water" => self.water *= op,
            "grass" => self.grass *= op,
            "electric" => self.electric *= op,
            "psychic" => self.psychic *= op,
            "ice" => self.ice *= op,
            "dragon" => self.dragon *= op,
            "dark" => self.dark *= op,
            "fairy" => self.fairy *= op,
            _ => return Err("Type not found for TypeChart::apply")
        };

        Ok(())
    }
}