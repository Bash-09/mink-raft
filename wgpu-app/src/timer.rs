use std::time::Instant;

pub struct Timer {
    last: Instant,
    fps: u32,
    last_delta: f64,
    tick_duration: f64,
    frame_count: u32,
    frame_time: f64,
    fps_update_time: f64,

    abs_time: f64,
}

/// Keeps track of timing
impl Timer {
    #[must_use]
    pub fn new() -> Self {
        Self {
            last: Instant::now(),
            fps: 0,
            last_delta: 0.0,
            tick_duration: 0.001,
            frame_count: 0,
            frame_time: 0.0,
            fps_update_time: 0.25,

            abs_time: 0.0,
        }
    }

    /// Reset time to 0
    pub fn reset(&mut self) {
        self.last = Instant::now();
        self.abs_time = 0.0;
    }

    /// Returns the time since `go()` last returned a value.
    /// If less than `frame_min_duration` has elapsed since this function last returned a value then it will return None,
    /// indicating it is not yet time for the next tick. Otherwise it will return `Some` containing how much time has elapsed in seconds
    pub fn go(&mut self) -> Option<f64> {
        let now = self.last.elapsed();
        #[allow(clippy::cast_precision_loss)]
        let delta = (now.as_micros() as f64) / 1_000_000.0;

        if delta < self.tick_duration {
            return None;
        }

        self.abs_time += self.last_delta;

        self.frame_count += 1;
        self.frame_time += delta;
        if self.frame_time > self.fps_update_time {
            #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
            let fps = (f64::from(self.frame_count) * (1.0 / self.frame_time)) as u32;
            self.fps = fps;
            self.frame_count = 0;
            self.frame_time = 0.0;
        }

        self.last_delta = delta;
        self.last = Instant::now();
        Some(delta)
    }

    /// Set how many seconds should pass before the next tick
    pub fn set_tick_duration(&mut self, dur: f64) {
        self.tick_duration = dur;
    }

    /// Set how often the fps count should be updated, shorter durations update the fps count more often but may be less accurate
    pub fn set_fps_update_time(&mut self, dur: f64) {
        self.fps_update_time = dur;
    }

    /// Approximate FPS
    #[must_use]
    pub const fn fps(&self) -> u32 {
        self.fps
    }

    /// How much time has passed between ticks (updated by calling `go`)
    #[must_use]
    pub const fn delta(&self) -> f64 {
        self.last_delta
    }

    /// How much time has passed since this Timer was created or `reset` was last called
    #[must_use]
    pub const fn absolute_time(&self) -> f64 {
        self.abs_time
    }
}

impl Default for Timer {
    fn default() -> Self {
        Self::new()
    }
}
