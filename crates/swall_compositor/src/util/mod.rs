use std::time::{Duration, Instant};

mod async_wayland_server;
pub use async_wayland_server::ListeningSocket;

/// Count the time it takes to render a frame
#[derive(Debug, Default)]
pub struct Counter(Option<Instant>);

impl Counter {
    /// Calculate time since [Counter::tick] was last called
    pub fn tick(&mut self) -> Duration {
        let last = self.0.take();
        let dur = if let Some(e) = last {
            e.elapsed()
        } else {
            Duration::ZERO
        };
        let now = Instant::now();
        self.0 = Some(now);
        dur
    }
}
