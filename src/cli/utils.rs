use owo_colors::{colors::xterm, Style};

pub struct StyleSheet {
    pub default: Style,
    pub header: Style,
    pub quad: Style,
    pub double: Style,
    pub neutral: Style,
    pub half: Style,
    pub quarter: Style,
    pub zero: Style,
    pub power: Style,
    pub accuracy: Style,
    pub pp: Style,
    pub hp: Style,
    pub attack: Style,
    pub defense: Style,
    pub special_attack: Style,
    pub special_defense: Style,
    pub speed: Style,
    pub move_: Style,
}

impl Default for StyleSheet {
    fn default() -> Self {
        Self {
            default: Style::new().default_color(),
            header: Style::new().bright_green().bold(),
            quad: Style::new().red(),
            double: Style::new().yellow(),
            neutral: Style::new().green(),
            half: Style::new().blue(),
            quarter: Style::new().bright_cyan(),
            zero: Style::new().purple(),
            power: Style::new().fg::<xterm::FlushOrange>(),
            accuracy: Style::new().fg::<xterm::FernGreen>(),
            pp: Style::new().fg::<xterm::ScienceBlue>(),
            hp: Style::new().fg::<xterm::GuardsmanRed>(),
            attack: Style::new().fg::<xterm::DecoOrange>(),
            defense: Style::new().fg::<xterm::AeroBlue>(),
            special_attack: Style::new().fg::<xterm::BlazeOrange>(),
            special_defense: Style::new().fg::<xterm::PoloBlue>(),
            speed: Style::new().fg::<xterm::PurplePizzazz>(),
            move_: Style::new().fg::<xterm::SpringGreen>(),
        }
    }
}

pub struct WeaknessGroups<T> {
    pub quad: Vec<T>,
    pub double: Vec<T>,
    pub neutral: Vec<T>,
    pub half: Vec<T>,
    pub quarter: Vec<T>,
    pub zero: Vec<T>,
    pub other: Vec<T>,
}

impl<T> WeaknessGroups<T> {
    pub fn new<C, F, I>(collection: C, mut cb: F) -> Self
    where
        C: IntoIterator<Item = I>,
        F: FnMut(I) -> Option<(T, f32)>,
    {
        let mut groups = WeaknessGroups {
            quad: vec![],
            double: vec![],
            neutral: vec![],
            half: vec![],
            quarter: vec![],
            zero: vec![],
            other: vec![],
        };

        for element in collection {
            if let Some(result) = cb(element) {
                let (item, multiplier) = result;
                match multiplier {
                    x if x == 4.0 => groups.quad.push(item),
                    x if x == 2.0 => groups.double.push(item),
                    x if x == 1.0 => groups.neutral.push(item),
                    x if x == 0.5 => groups.half.push(item),
                    x if x == 0.25 => groups.quarter.push(item),
                    x if x == 0.0 => groups.zero.push(item),
                    _ => groups.other.push(item),
                }
            }
        }

        groups
    }
}
