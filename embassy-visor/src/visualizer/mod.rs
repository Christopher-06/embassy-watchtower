use anyhow::Context;
use crossbeam::channel::Receiver;
use crossterm::event::KeyEvent;
use ratatui::{style::{Color, Stylize}, text::Line};

use crate::{
    tracing::{instance::TracingInstance, stats::instance_stats::InstanceStats},
    visualizer::app::App,
};

pub mod app;
mod views;

pub enum TuiAppEvent {
    KeyPressed(KeyEvent),
    TraceStatistics(InstanceStats),
    NewLogLine(String)
}

pub fn run_main_tui(instance: TracingInstance, logs_recver : Receiver<String>) -> anyhow::Result<()> {
    let mut terminal = ratatui::init();
    let app_result = App::new(instance, logs_recver)
        .context("Error creating TUI App")?
        .run(&mut terminal)
        .context("Failed running ratatui app");

    ratatui::restore();
    app_result
}

pub fn cpu_usage_colors(cpu_utilization: f32) -> Color {
    match cpu_utilization {
        x if x > 70.0 => Color::Red,
        x if x > 40.0 => Color::Yellow,
        _ => Color::Blue,
    }
}

/// Recolors defmt log messages based on their log level tags:
/// [INFO] Hello World
/// - BLUE - gray
pub fn recolor_defmt_messages(message: &String) -> Line {
    let closing_bracket_pos = message.find(']').unwrap_or(0);
    let text = &message[closing_bracket_pos + 1..].trim_start();

    if message.starts_with("[ERROR") {
        format!("[ERROR]").red() + format!(" {}", text).gray()
    } else if message.starts_with("[WARN") {
        format!("[WARN]").yellow() + format!(" {}", text).gray()
    } else if message.starts_with("[INFO") {
        format!("[INFO]").blue() + format!(" {}", text).gray()
    } else if message.starts_with("[DEBUG") {
        format!("[DEBUG]").green() + format!(" {}", text).gray()
    } else {
        Line::from(message.to_string().gray())
    }
}