use std::{
    sync::{
        atomic::{AtomicU8, Ordering},
        Arc,
    },
    time::Instant,
};

/// Handles the updating of the `Chip8` sound and delay timers. The `delay_timer`  and
/// the `sound_timer` are decremented by `1` at a rate of `60Hz`.
#[cfg_attr(feature = "persistence", derive(serde::Serialize, serde::Deserialize))]
pub struct Clock {
    pub delay_timer: u8,
    #[cfg_attr(feature = "persistence", serde(skip))]
    pub sound_timer: Arc<AtomicU8>,
    pub vblank_interrupt: bool,
    #[cfg_attr(feature = "persistence", serde(skip, default = "Instant::now"))]
    last_delay: Instant,
}

impl Default for Clock {
    fn default() -> Self {
        Self {
            delay_timer: Default::default(),
            sound_timer: Default::default(),
            last_delay: Instant::now(),
            vblank_interrupt: Default::default(),
        }
    }
}

impl Clock {
    /// Create a new [`Clock`].
    pub fn new() -> Self {
        Self::default()
    }

    /// Update the delay and sound timers.
    pub fn update(&mut self) {
        if self.last_delay.elapsed().as_secs_f32() >= (1.0 / 60.0) {
            self.delay_timer -= if self.delay_timer > 0 { 1 } else { 0 };

            if self.sound_timer.load(Ordering::SeqCst) > 0 {
                self.sound_timer.fetch_sub(1, Ordering::SeqCst);
            }

            self.vblank_interrupt = true;
            self.last_delay = Instant::now();
        } else {
            self.vblank_interrupt = false;
        }
    }
}
