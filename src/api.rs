use std::collections::HashMap;

use anyhow::Result;
use rustemon::client::RustemonClient;
use rustemon::model::moves::Move as RustemonMove;
use rustemon::moves as rustemon_moves;
use rustemon::pokemon::{pokemon as rustemon_pokemon, type_ as rustemon_type};

use crate::pokemon::{Move, Pokemon, Type, TypeChart};

pub struct ApiWrapper {
    client: RustemonClient,
}

impl Default for ApiWrapper {
    fn default() -> ApiWrapper {
        ApiWrapper {
            client: RustemonClient::default(),
        }
    }
}
impl ApiWrapper {
    pub async fn get_pokemon(&self, pokemon: &str, version: &str) -> Result<Pokemon> {
        let pokemon = rustemon_pokemon::get_by_name(&pokemon, &self.client).await?;
        let name = pokemon.name;

        let mut types = pokemon.types.iter();
        let primary_type = types.find(|t| t.slot == 1).unwrap().type_.name.clone();
        let secondary_type = match types.find(|t| t.slot == 2) {
            Some(t) => Some(t.type_.name.clone()),
            None => None,
        };

        let mut moves = HashMap::new();
        pokemon.moves.iter().for_each(|mv| {
            let version_group = mv
                .version_group_details
                .iter()
                .find(|vg| vg.version_group.name == version);
            if let Some(vg) = version_group {
                moves.insert(
                    mv.move_.name.clone(),
                    (
                        vg.move_learn_method.name.clone(),
                        vg.level_learned_at.clone(),
                    ),
                );
            }
        });

        Ok(Pokemon {
            name,
            primary_type,
            secondary_type,
            moves,
            version: version.to_string(),
            api: self,
        })
    }

    pub async fn get_type(&self, type_str: &str) -> Result<Type> {
        let type_ = rustemon_type::get_by_name(type_str, &self.client).await?;
        let mut offense_chart = HashMap::new();
        let mut defense_chart = HashMap::new();

        type_.damage_relations.no_damage_to.iter().for_each(|t| {
            offense_chart.insert(t.name.to_string(), 0.0);
        });
        type_.damage_relations.half_damage_to.iter().for_each(|t| {
            offense_chart.insert(t.name.to_string(), 0.5);
        });
        type_
            .damage_relations
            .double_damage_to
            .iter()
            .for_each(|t| {
                offense_chart.insert(t.name.to_string(), 2.0);
            });

        type_.damage_relations.no_damage_from.iter().for_each(|t| {
            defense_chart.insert(t.name.to_string(), 0.0);
        });
        type_
            .damage_relations
            .half_damage_from
            .iter()
            .for_each(|t| {
                defense_chart.insert(t.name.to_string(), 0.5);
            });
        type_
            .damage_relations
            .double_damage_from
            .iter()
            .for_each(|t| {
                defense_chart.insert(t.name.to_string(), 2.0);
            });

        Ok(Type {
            name: type_.name,
            offense_chart: TypeChart::from_hashmap(offense_chart),
            defense_chart: TypeChart::from_hashmap(defense_chart),
            api: self,
        })
    }

    pub async fn get_move(&self, name: &str) -> Result<Move> {
        let RustemonMove {
            name,
            accuracy,
            power,
            pp,
            damage_class,
            type_,
            effect_entries,
            ..
        } = rustemon_moves::move_::get_by_name(name, &self.client).await?;
        let effect_entry = effect_entries
            .into_iter()
            .find(|e| e.language.name == "en")
            .unwrap_or_default();

        Ok(Move {
            name,
            accuracy,
            power,
            pp,
            damage_class: damage_class.name,
            type_: type_.name,
            effect: effect_entry.effect,
            effect_short: effect_entry.short_effect,
            api: self,
        })
    }
}
