use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Layout, Rect},
    style::{Styled, Stylize},
    text::Line,
    widgets::{Block, Borders, Widget},
};

use crate::{
    tracing::stats::core_stats::CoreStats,
    visualizer::{cpu_usage_colors, views::executor_view::ExecutorView},
};

pub struct CoreView<'a>(pub &'a CoreStats);

impl<'a> CoreView<'a> {
    pub fn get_min_height(&self) -> u16 {
        // Minimum height is 1 (for border) + sum of executor min heights
        2 + self
            .0
            .executors
            .iter()
            .map(|e| {
                let executor_view = ExecutorView(e);
                executor_view.get_min_height()
            })
            .sum::<u16>()
    }
}

impl<'a> Widget for &'a CoreView<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let mut title = Line::from(format!(" Core {} ", self.0.core_id).bold());

        // Add CPU Utilization when more than two executors
        if self.0.executors.len() > 1 {
            title += format!(" ( {:.2}% ) ", self.0.cpu_utilization_percent)
                .set_style(cpu_usage_colors(self.0.cpu_utilization_percent));
        }

        let block = Block::new().borders(Borders::ALL).title(title);
        let block_inner = block.inner(area);

        let chunks = Layout::default()
            .constraints(
                self.0
                    .executors
                    .iter()
                    .map(|e| Constraint::Length(ExecutorView(e).get_min_height()))
                    .collect::<Vec<_>>(),
            )
            .split(block_inner);

        // Render each executor view
        for (executor_stat, chunk) in self.0.executors.iter().zip(chunks.to_vec()) {
            let executor_view = ExecutorView(executor_stat);
            executor_view.render(chunk, buf);
        }

        block.render(area, buf);
    }
}
