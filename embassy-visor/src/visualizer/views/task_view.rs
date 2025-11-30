use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Layout, Rect},
    style::Stylize,
    text::Line,
    widgets::{Gauge, Paragraph, Widget},
};

use crate::{tracing::stats::task_stats::TaskStats, visualizer::cpu_usage_colors};

pub struct TaskView<'a>(pub &'a TaskStats);

impl<'a> TaskView<'a> {}

impl<'a> Widget for &'a TaskView<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let chunks = Layout::default()
            .constraints(vec![Constraint::Length(50), Constraint::Percentage(100)])
            .direction(ratatui::layout::Direction::Horizontal)
            .split(area)
            .to_vec();

        Paragraph::new(Line::from(format!("{}", self.0.name).bold())).render(chunks[0], buf);

        // Map colors
        let label = format!("{:>5.2}%", self.0.cpu_utilization_percent);
        Gauge::default()
            .gauge_style(cpu_usage_colors(self.0.cpu_utilization_percent))
            .ratio(self.0.cpu_utilization_percent as f64 / 100.0)
            .label(label)
            .render(chunks[1], buf);
    }
}
