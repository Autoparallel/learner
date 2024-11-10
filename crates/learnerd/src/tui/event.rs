//! Event handling for the TUI.
//!
//! This module provides event handling for keyboard input and terminal events.
//! It uses a channel-based approach to handle events asynchronously.

use std::time::Duration;

use crossterm::event::{self, Event as CrosstermEvent, KeyEvent, KeyEventKind};
use tokio::sync::mpsc;

/// Events that can occur in the application
#[derive(Debug)]
pub enum Event {
  /// Key press events
  Key(KeyEvent),
  /// Terminal resize events
  Resize(u16, u16),
  /// Timer tick for regular updates
  Tick,
}

/// Event handler that manages input and timer events
pub struct EventHandler {
  /// Sender half of event channel
  _tx: mpsc::Sender<Event>,
  /// Receiver half of event channel
  rx:  mpsc::Receiver<Event>,
}

impl EventHandler {
  /// Creates a new event handler.
  ///
  /// # Arguments
  /// * `tick_rate` - How often to send tick events
  pub fn new(tick_rate: Duration) -> Self {
    let (tx, rx) = mpsc::channel(100);
    let event_tx = tx.clone();

    // Spawn event handling loop
    tokio::spawn(async move {
      let mut interval = tokio::time::interval(tick_rate);
      loop {
        // Check for events with a timeout
        if event::poll(Duration::from_millis(250)).unwrap() {
          match event::read().unwrap() {
            CrosstermEvent::Key(key) =>
              if key.kind == KeyEventKind::Press {
                event_tx.send(Event::Key(key)).await.unwrap();
              },
            CrosstermEvent::Resize(width, height) => {
              event_tx.send(Event::Resize(width, height)).await.unwrap();
            },
            _ => {},
          }
        }

        // Send tick event
        interval.tick().await;
        event_tx.send(Event::Tick).await.unwrap();
      }
    });

    Self { _tx: tx, rx }
  }

  /// Receives the next event from the event handler.
  ///
  /// This method will await the next event that occurs.
  pub async fn next(&mut self) -> Option<Event> { self.rx.recv().await }
}
