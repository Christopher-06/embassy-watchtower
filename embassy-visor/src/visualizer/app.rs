use std::{
    collections::VecDeque,
    io,
    sync::atomic::{AtomicUsize, Ordering},
    time::Duration,
};

use crossbeam::channel::{self, Receiver, Sender};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::{
    DefaultTerminal, Frame,
    buffer::Buffer,
    layout::{Constraint, Layout, Margin, Rect},
    style::Stylize,
    symbols::border,
    text::Line,
    widgets::{Block, Borders, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, Widget},
};

use crate::{
    tracing::{instance::TracingInstance, stats::instance_stats::InstanceStats},
    visualizer::{TuiAppEvent, recolor_defmt_messages, views::instance_view::InstanceView},
};

pub static MAX_LOG_LINES: AtomicUsize = AtomicUsize::new(100);

#[derive(Debug)]
pub struct App {
    exit: bool,
    instance_stats: InstanceStats,
    log_lines: VecDeque<String>,
    log_scroll: u16,

    event_recver: Receiver<TuiAppEvent>,
}

impl App {
    pub fn new(instance: TracingInstance, logs_recver: Receiver<String>) -> anyhow::Result<Self> {
        // Start Event Listener
        let (event_sender, event_recver) = channel::unbounded();
        {
            let event_sender = event_sender.clone();
            let _ = std::thread::spawn(move || run_keyevent_listener(event_sender.clone()));
        }
        {
            let event_sender = event_sender.clone();
            let _ = std::thread::spawn(move || run_instance_stats_gatherer(event_sender, instance));
        }
        {
            let event_sender = event_sender.clone();
            let _ = std::thread::spawn(move || run_log_line_listener(event_sender, logs_recver));
        }

        Ok(Self {
            instance_stats: InstanceStats::default(),
            exit: false,
            log_lines: VecDeque::with_capacity(MAX_LOG_LINES.load(Ordering::Relaxed)),
            event_recver,
            log_scroll: 0,
        })
    }

    /// runs the application's main loop until the user quits
    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> io::Result<()> {
        while !self.exit {
            terminal.draw(|frame| self.draw(frame))?;
            self.handle_events()?;
        }
        Ok(())
    }

    fn exit(&mut self) {
        self.exit = true;
    }

    fn on_new_stats(&mut self, new_stats: InstanceStats) {
        self.instance_stats = new_stats;
    }

    fn on_new_log_line(&mut self, new_line: String) {
        self.log_lines.push_back(new_line);

        // Adjust scroll to stay at bottom if we were already at bottom
        if self.log_scroll > self.log_lines.len().saturating_sub(5) as u16 {
            self.log_scroll = self
                .log_scroll
                .saturating_add(1)
                .min(self.log_lines.len().saturating_sub(3) as u16);
        }

        let max_log_lines = MAX_LOG_LINES.load(Ordering::Relaxed);
        while self.log_lines.len() > max_log_lines {
            let _ = self.log_lines.pop_front();
        }
    }

    fn handle_key_event(&mut self, key_event: KeyEvent) {
        match key_event.code {
            KeyCode::Char('q') | KeyCode::Esc => self.exit(),
            KeyCode::Char('c') if key_event.modifiers.contains(KeyModifiers::CONTROL) => {
                self.exit()
            }
            KeyCode::Up => self.log_scroll = self.log_scroll.saturating_sub(1).max(0),
            KeyCode::Down => {
                self.log_scroll = self
                    .log_scroll
                    .saturating_add(1)
                    .min(self.log_lines.len().saturating_sub(3) as u16)
            }
            _ => {}
        }
    }

    fn handle_events(&mut self) -> io::Result<()> {
        if let Ok(tui_event) = self.event_recver.recv() {
            match tui_event {
                TuiAppEvent::KeyPressed(key_event) => self.handle_key_event(key_event),
                TuiAppEvent::TraceStatistics(new_stats) => self.on_new_stats(new_stats),
                TuiAppEvent::NewLogLine(new_line) => self.on_new_log_line(new_line),
            }
        }
        Ok(())
    }

    fn draw(&self, frame: &mut Frame) {
        let layout = Layout::default()
            .constraints(
                [
                    Constraint::Length(InstanceView(&self.instance_stats).get_min_height() + 2),
                    Constraint::Min(6),
                ]
                .as_ref(),
            )
            .split(frame.area());

        frame.render_widget(self, layout[0]);

        let vertical_scroll = self.log_scroll; // from app state

        let items = self
            .log_lines
            .iter()
            .map(recolor_defmt_messages)
            .chain([Line::from("")])
            .collect::<Vec<_>>();
        let paragraph: Paragraph<'_> = Paragraph::new((items).clone())
            .scroll((vertical_scroll as u16, 0))
            .block(Block::new().borders(Borders::ALL).title("Logs")); // to show a background for the scrollbar

        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("↑"))
            .end_symbol(Some("↓"));

        let mut scrollbar_state =
            ScrollbarState::new(items.len()).position(vertical_scroll as usize);

        // let area = frame.area();
        let area = layout[1];
        // Note we render the paragraph
        frame.render_widget(paragraph, area);
        // and the scrollbar, those are separate widgets
        frame.render_stateful_widget(
            scrollbar,
            area.inner(Margin {
                // using an inner vertical margin of 1 unit makes the scrollbar inside the block
                vertical: 1,
                horizontal: 0,
            }),
            &mut scrollbar_state,
        );
    }
}

impl Widget for &App {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let title = Line::from(" Embassy Visor - Watchtower ".bold());
        let instructions = Line::from(vec![
            // " Settings ".into(),
            // "<S>".blue().bold(),
            // " Quit ".into(),
            // "<Q/ESC> ".blue().bold(),
        ]);
        let block = Block::bordered()
            .title(title.centered())
            .title_bottom(instructions.centered())
            .border_set(border::THICK);

        let inner_block = block.inner(area);

        InstanceView(&self.instance_stats).render(inner_block, buf);

        block.render(area, buf);
    }
}

fn run_keyevent_listener(event_sender: Sender<TuiAppEvent>) {
    loop {
        let event = match event::read() {
            Ok(Event::Key(key_event)) if key_event.kind == KeyEventKind::Press => {
                TuiAppEvent::KeyPressed(key_event)
            }
            _ => continue,
        };

        let result = event_sender.send(event);

        if result.is_err() {
            break; // channel clossed
        }
    }
}

fn run_log_line_listener(event_sender: Sender<TuiAppEvent>, logs_recver: Receiver<String>) {
    loop {
        match logs_recver.recv() {
            Ok(new_line) => {
                let result = event_sender.send(TuiAppEvent::NewLogLine(new_line));

                if result.is_err() {
                    break; // Event Channel closed
                }
            }
            Err(_) => break, // Log Channel closed
        }
    }
}

fn run_instance_stats_gatherer(event_sender: Sender<TuiAppEvent>, instance: TracingInstance) {
    loop {
        std::thread::sleep(Duration::from_millis(100));

        let new_stats = instance.get_stats();
        let result = event_sender.send(TuiAppEvent::TraceStatistics(new_stats));
        if result.is_err() {
            break; // channel closed
        }
    }
}
