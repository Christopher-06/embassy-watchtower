use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Layout, Rect},
    widgets::Widget,
};

use crate::{
    tracing::stats::instance_stats::InstanceStats, visualizer::views::core_view::CoreView,
};

pub struct InstanceView<'a>(pub &'a InstanceStats);

impl<'a> InstanceView<'a> {
    pub fn get_min_height(&self) -> u16 {
        // Minimum height is 2 (for border) + sum of core view heights + spacing
        let core_heights: u16 = self
            .0
            .core_stats
            .iter()
            .map(|c| CoreView(c).get_min_height())
            .sum();
        let spacing = if self.0.core_stats.is_empty() {
            0
        } else {
            self.0.core_stats.len() as u16 - 1
        };
     core_heights + spacing
    }
}

impl Widget for &InstanceView<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let chunks = Layout::default()
            .constraints(
                self.0
                    .core_stats
                    .iter()
                    .map(|c| Constraint::Length(CoreView(c).get_min_height()))
                    .collect::<Vec<_>>(),
            )
            // .spacing(1)
            .split(area);

        // Render each core view
        for (core_stat, chunk) in self.0.core_stats.iter().zip(chunks.to_vec()) {
            let core_view = CoreView(core_stat);
            core_view.render(chunk, buf);
        }
    }
}
