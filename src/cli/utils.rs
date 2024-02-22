use super::{Colors, Effects, Style};

use indoc::formatdoc;
use std::io::{stdout, IsTerminal};

pub trait DisplayComponent: std::fmt::Display {
    fn style(&self) -> Style {
        Style::new(self.color_enabled())
    }

    fn color(&self, color: Colors) -> anstyle::Style {
        self.style().fg(color).ansi()
    }

    fn color_effect(&self, color: Colors, effect: Effects) -> anstyle::Style {
        self.style().fg(color).effect(effect).ansi()
    }

    fn color_enabled(&self) -> bool;
}

pub fn is_color_enabled() -> bool {
    if let Ok(force_color) = std::env::var("FORCE_COLOR") {
        if is_env_affirmative(&force_color) {
            return true;
        }
    };
    if let Ok(no_color) = std::env::var("NO_COLOR") {
        if is_env_affirmative(&no_color) {
            return false;
        }
    };

    is_terminal()
}

pub fn is_env_negative(value: &str) -> bool {
    let value = value.to_lowercase();
    value == "false" || value == "no" || value == "0"
}

pub fn is_env_affirmative(value: &str) -> bool {
    !is_env_negative(value)
}

pub fn is_terminal() -> bool {
    stdout().is_terminal()
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
            double = self.format_group("double", weakness_groups.double, Colors::Orange);
        }
        if !weakness_groups.neutral.is_empty() {
            neutral = self.format_group("neutral", weakness_groups.neutral, Colors::Green);
        }
        if !weakness_groups.half.is_empty() {
            half = self.format_group("half", weakness_groups.half, Colors::Cyan);
        }
        if !weakness_groups.quarter.is_empty() {
            quarter = self.format_group("quarter", weakness_groups.quarter, Colors::Blue);
        }
        if !weakness_groups.zero.is_empty() {
            zero = self.format_group("zero", weakness_groups.zero, Colors::Violet);
        }
        if !weakness_groups.other.is_empty() {
            other = self.format_group("other", weakness_groups.other, Colors::Yellow);
        }

        let output = formatdoc! {
            "{quad}{double}{neutral}{half}{quarter}{zero}{other}"
        };

        if !output.is_empty() {
            output
        } else {
            String::from("\nNone")
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn colors_rate() {
        // Test when number is greater than 83% of the ceiling
        assert_eq!(Colors::Red, Colors::rate(84, 100));

        // Test when number is between 66% and 83% of the ceiling
        assert_eq!(Colors::Orange, Colors::rate(70, 100));

        // Test when number is between 50% and 66% of the ceiling
        assert_eq!(Colors::Yellow, Colors::rate(60, 100));

        // Test when number is between 33% and 50% of the ceiling
        assert_eq!(Colors::Green, Colors::rate(40, 100));

        // Test when number is between 16% and 33% of the ceiling
        assert_eq!(Colors::Blue, Colors::rate(20, 100));

        // Test when number is less than 16% of the ceiling
        assert_eq!(Colors::Violet, Colors::rate(10, 100));
    }

    #[test]
    fn colors_ansi() {
        let orange = Style::new(false).fg(Colors::Orange).ansi();
        assert_eq!("plain text", format!("{orange}plain text{orange:#}"));

        let red = Style::new(true).fg(Colors::Red).ansi();
        assert_eq!(
            "\u{1b}[38;5;160mRed\u{1b}[0mRum",
            format!("{red}Red{red:#}Rum")
        );

        let header_bold = Style::new(true)
            .fg(Colors::Header)
            .effect(Effects::Bold)
            .ansi();
        assert_eq!(
            "\u{1b}[1m\u{1b}[38;5;10mheader\u{1b}[0m",
            format!("{header_bold}header{header_bold:#}")
        );

        let blue_bg = Style::new(true).bg(Colors::Blue).ansi();
        assert_eq!(
            "lucy in the \u{1b}[48;5;33msky\u{1b}[0m",
            format!("lucy in the {blue_bg}sky{blue_bg:#}")
        );
    }
}
