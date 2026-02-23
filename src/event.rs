use crate::types::RepoStatus;
use crossterm::event::{self, Event as CrosstermEvent, KeyEvent};
use std::time::Duration;
use tokio::sync::mpsc;

#[derive(Debug)]
pub enum Event {
    Key(KeyEvent),
    Tick,
    ScanComplete(Vec<RepoStatus>),
}

pub fn spawn_event_reader(tx: mpsc::UnboundedSender<Event>, tick_rate_ms: u64) {
    std::thread::spawn(move || loop {
        if event::poll(Duration::from_millis(tick_rate_ms)).unwrap_or(false) {
            if let Ok(evt) = event::read() {
                match evt {
                    CrosstermEvent::Key(key) => {
                        if tx.send(Event::Key(key)).is_err() {
                            return;
                        }
                    }
                    _ => {}
                }
            }
        } else {
            // Tick event
            if tx.send(Event::Tick).is_err() {
                return;
            }
        }
    });
}
