use super::{Colors, DisplayComponent, WeaknessDisplay};
use crate::pokemon::TypeChart;

use std::fmt;

use indoc::writedoc;

pub struct TypeChartComponent<'a> {
    pub type_chart: &'a TypeChart,
    pub label: &'a str,
}

impl fmt::Display for DisplayComponent<TypeChartComponent<'_>> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let TypeChartComponent { type_chart, label } = self.context;

        let weakness_groups = self.group_by_weakness(type_chart.get_value(), |item| {
            Some((item.0.clone(), *item.1))
        });
        let type_chart = self.format_groups(weakness_groups);

        writedoc! {
            f,
            "{header}{label}{header:#}{type_chart}",
            header = self.ansi_bold(Colors::Header),
        }
    }
}

impl WeaknessDisplay<String> for DisplayComponent<TypeChartComponent<'_>> {
    fn format_group(&self, label: &'static str, mut types: Vec<String>, color: Colors) -> String {
        types.sort();
        let style = self.ansi(color);
        format!("\n{label}: {style}{}{style:#}", types.join(" "))
    }
}
