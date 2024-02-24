#![allow(unused_imports)]

use super::{Colors, DisplayComponent};
use crate::pokemon::PokemonData;

use std::fmt;

use indoc::writedoc;

pub struct CoverageComponent<'a, 'b> {
    pub pokemon: &'a Vec<PokemonData<'b>>,
}

impl fmt::Display for DisplayComponent<CoverageComponent<'_, '_>> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "meow")
    }
}
