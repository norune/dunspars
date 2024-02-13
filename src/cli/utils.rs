use std::io::{stdout, IsTerminal};

use indoc::formatdoc;

pub enum Colors {
    Header,
    Red,
    Yellow,
    Green,
    Blue,
    Cyan,
    Violet,
}

impl Colors {
    fn get(&self) -> Option<anstyle::Color> {
        match self {
            Colors::Header => Some(anstyle::Ansi256Color(10).into()),
            Colors::Red => Some(anstyle::Ansi256Color(160).into()),
            Colors::Yellow => Some(anstyle::Ansi256Color(184).into()),
            Colors::Green => Some(anstyle::Ansi256Color(77).into()),
            Colors::Blue => Some(anstyle::Ansi256Color(33).into()),
            Colors::Cyan => Some(anstyle::Ansi256Color(45).into()),
            Colors::Violet => Some(anstyle::Ansi256Color(99).into()),
        }
    }
}

pub enum Effects {
    Bold,
    Underline,
}

impl Effects {
    fn get(&self) -> anstyle::Effects {
        match self {
            Effects::Bold => anstyle::Effects::BOLD,
            Effects::Underline => anstyle::Effects::UNDERLINE,
        }
    }
}

pub struct Style {
    style: anstyle::Style,
    color_enabled: bool,
}

impl Style {
    pub fn new(color_enabled: bool) -> Self {
        Self {
            style: anstyle::Style::new(),
            color_enabled,
        }
    }

    pub fn fg(mut self, color: Colors) -> Self {
        if self.color_enabled {
            self.style = self.style.fg_color(color.get());
        }
        self
    }

    #[allow(dead_code)]
    pub fn bg(mut self, color: Colors) -> Self {
        if self.color_enabled {
            self.style = self.style.bg_color(color.get());
        }
        self
    }

    pub fn effect(mut self, effect: Effects) -> Self {
        if self.color_enabled {
            self.style = self.style.effects(effect.get());
        }
        self
    }

    pub fn ansi(&self) -> anstyle::Style {
        self.style
    }
}

pub trait DisplayComponent: std::fmt::Display {
    fn color(&self) -> Style {
        Style::new(self.color_enabled())
    }

    fn fg(&self, color: Colors) -> anstyle::Style {
        self.color().fg(color).ansi()
    }

    fn fg_effect(&self, color: Colors, effect: Effects) -> anstyle::Style {
        self.color().fg(color).effect(effect).ansi()
    }

    fn color_enabled(&self) -> bool;
}

pub fn is_color_enabled() -> bool {
    if let Ok(force_color) = std::env::var("FORCE_COLOR") {
        if !force_color.is_empty() {
            return true;
        }
    };
    if let Ok(no_color) = std::env::var("NO_COLOR") {
        if !no_color.is_empty() {
            return false;
        }
    };

    is_terminal()
}

fn is_terminal() -> bool {
    let stdout = stdout();
    stdout.is_terminal()
}

pub trait WeaknessDisplay<T> {
    fn group_by_weakness<C, F, I>(&self, collection: C, mut cb: F) -> WeaknessGroups<T>
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

    fn format_groups(&self, weakness_groups: WeaknessGroups<T>) -> String {
        let mut quad = String::from("");
        let mut double = String::from("");
        let mut neutral = String::from("");
        let mut half = String::from("");
        let mut quarter = String::from("");
        let mut zero = String::from("");
        let mut other = String::from("");

        if !weakness_groups.quad.is_empty() {
            quad = self.format_group("quad", weakness_groups.quad, Colors::Red);
        }
        if !weakness_groups.double.is_empty() {
            double = self.format_group("double", weakness_groups.double, Colors::Yellow);
        }
        if !weakness_groups.neutral.is_empty() {
            neutral = self.format_group("neutral", weakness_groups.neutral, Colors::Green);
        }
        if !weakness_groups.half.is_empty() {
            half = self.format_group("half", weakness_groups.half, Colors::Blue);
        }
        if !weakness_groups.quarter.is_empty() {
            quarter = self.format_group("quarter", weakness_groups.quarter, Colors::Cyan);
        }
        if !weakness_groups.zero.is_empty() {
            zero = self.format_group("zero", weakness_groups.zero, Colors::Violet);
        }
        if !weakness_groups.other.is_empty() {
            other = self.format_group("other", weakness_groups.other, Colors::Green);
        }

        formatdoc! {
            "{quad}{double}{neutral}{half}{quarter}{zero}{other}"
        }
    }

    fn format_group(&self, label: &'static str, group: Vec<T>, color: Colors) -> String;
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
