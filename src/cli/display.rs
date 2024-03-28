mod ability;
mod coverage;
mod evolution_step;
mod match_;
mod move_;
mod move_list;
mod move_weakness;
mod pokemon;
mod stats;
mod typechart;
mod weakness;

pub use coverage::CoverageComponent;
pub use match_::MatchComponent;
pub use move_list::MoveListComponent;
pub use move_weakness::MoveWeaknessComponent;
pub use typechart::TypeChartComponent;
use weakness::WeaknessDisplay;

use super::utils::is_color_enabled;

pub struct DisplayComponent<T> {
    context: T,
    color_enabled: Option<bool>,
}

impl<T> DisplayComponent<T> {
    pub fn new(context: T, color_enabled: Option<bool>) -> Self {
        Self {
            context,
            color_enabled,
        }
    }

    fn is_color_enabled(&self) -> bool {
        self.color_enabled.unwrap_or(is_color_enabled())
    }

    fn style(&self) -> Style {
        Style::new(self.is_color_enabled())
    }

    fn ansi(&self, color: Colors) -> anstyle::Style {
        self.style().fg(color).ansi()
    }

    fn ansi_bold(&self, color: Colors) -> anstyle::Style {
        self.style().fg(color).effect(Effects::Bold).ansi()
    }

    #[allow(dead_code)]
    fn ansi_underline(&self, color: Colors) -> anstyle::Style {
        self.style().fg(color).effect(Effects::Underline).ansi()
    }
}

#[derive(Debug, PartialEq)]
enum Colors {
    Header,
    Red,
    Orange,
    Yellow,
    Green,
    Cyan,
    Blue,
    Violet,
}

impl Colors {
    fn rate(number: i64, ceiling: i64) -> Self {
        let number = number as f64;
        let ceiling = ceiling as f64;

        match number {
            number if number > ceiling * 0.83 => Colors::Red,
            number if number > ceiling * 0.66 => Colors::Orange,
            number if number > ceiling * 0.50 => Colors::Yellow,
            number if number > ceiling * 0.33 => Colors::Green,
            number if number > ceiling * 0.16 => Colors::Blue,
            _ => Colors::Violet,
        }
    }

    fn get(&self) -> Option<anstyle::Color> {
        match self {
            Colors::Header => Some(anstyle::Ansi256Color(10).into()),
            Colors::Red => Some(anstyle::Ansi256Color(160).into()),
            Colors::Orange => Some(anstyle::Ansi256Color(172).into()),
            Colors::Yellow => Some(anstyle::Ansi256Color(184).into()),
            Colors::Green => Some(anstyle::Ansi256Color(77).into()),
            Colors::Cyan => Some(anstyle::Ansi256Color(43).into()),
            Colors::Blue => Some(anstyle::Ansi256Color(33).into()),
            Colors::Violet => Some(anstyle::Ansi256Color(99).into()),
        }
    }
}

enum Effects {
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

struct Style {
    style: anstyle::Style,
    color_enabled: bool,
}

impl Style {
    fn new(color_enabled: bool) -> Self {
        Self {
            style: anstyle::Style::new(),
            color_enabled,
        }
    }

    fn fg(mut self, color: Colors) -> Self {
        if self.color_enabled {
            self.style = self.style.fg_color(color.get());
        }
        self
    }

    #[allow(dead_code)]
    fn bg(mut self, color: Colors) -> Self {
        if self.color_enabled {
            self.style = self.style.bg_color(color.get());
        }
        self
    }

    fn effect(mut self, effect: Effects) -> Self {
        if self.color_enabled {
            self.style = self.style.effects(effect.get());
        }
        self
    }

    fn ansi(&self) -> anstyle::Style {
        self.style
    }
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
