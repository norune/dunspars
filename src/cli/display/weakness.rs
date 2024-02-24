use super::Colors;
use indoc::formatdoc;

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
