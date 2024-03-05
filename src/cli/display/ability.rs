use super::{Colors, DisplayComponent};
use crate::data::Ability;

use std::fmt;

use indoc::writedoc;

impl fmt::Display for DisplayComponent<&Ability> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let Ability { name, effect, .. } = self.context;

        writedoc! {
            f,
            "{header}{name}{header:#}
            {effect}",
            header = self.ansi_bold(Colors::Header)
        }
    }
}
