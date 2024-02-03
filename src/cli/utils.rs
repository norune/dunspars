use owo_colors::{colors::xterm, Style};

pub struct StyleSheet {
    pub default: Style,
    pub header: Style,
    pub accent_red: Style,
    pub accent_yellow: Style,
    pub accent_green: Style,
    pub accent_blue: Style,
    pub accent_cyan: Style,
    pub accent_violet: Style,
}

impl Default for StyleSheet {
    fn default() -> Self {
        Self {
            default: Style::new().default_color(),
            header: Style::new().bright_green().bold(),
            accent_red: Style::new().fg::<xterm::GuardsmanRed>(),
            accent_yellow: Style::new().fg::<xterm::DollyYellow>(),
            accent_green: Style::new().fg::<xterm::FernGreen>(),
            accent_blue: Style::new().fg::<xterm::BlueRibbon>(),
            accent_cyan: Style::new().fg::<xterm::AeroBlue>(),
            accent_violet: Style::new().fg::<xterm::DarkHeliotropePurple>(),
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
