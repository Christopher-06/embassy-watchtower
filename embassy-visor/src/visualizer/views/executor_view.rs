use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Layout, Rect},
    style::{Styled, Stylize},
    text::Line,
    widgets::{Block, Borders, Padding, Widget},
};

use crate::{
    tracing::stats::executor_stats::ExecutorStats,
    visualizer::{cpu_usage_colors, views::task_view::TaskView},
};

pub struct ExecutorView<'a>(pub &'a ExecutorStats);

impl<'a> ExecutorView<'a> {
    pub fn get_min_height(&self) -> u16 {
        // Minimum height is 1 (for border) + number of tasks
        1 + self.0.tasks.len() as u16
    }
}

impl<'a> Widget for &'a ExecutorView<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let mut title = Line::from(format!("   {} ", self.0.name).bold());

        // Add CPU Utilization when more than two tasks
        if self.0.tasks.len() > 1 {
            title += format!(" ( {:.2}% ) ", self.0.cpu_utilization_percent)
                .set_style(cpu_usage_colors(self.0.cpu_utilization_percent));
        }

        let block = Block::new()
            .borders(Borders::TOP)
            .title(title)
            .padding(Padding::left(5));
        let block_inner = block.inner(area);

        let chunks = Layout::default()
            .constraints(
                self.0
                    .tasks
                    .iter()
                    .map(|_| Constraint::Length(1))
                    .collect::<Vec<_>>(),
            )
            .split(block_inner);

        // Render each task
        for (task_stat, chunk) in self.0.tasks.iter().zip(chunks.to_vec()) {
            TaskView(task_stat).render(chunk, buf);
        }

        block.render(area, buf);
    }
}
