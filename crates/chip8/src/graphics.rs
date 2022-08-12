pub const WIDTH: usize = 64;
pub const HEIGHT: usize = 32;
pub const PIXEL_COUNT: usize = WIDTH * HEIGHT;
pub const DEFAULT_FOREGROUND: RGB8 = RGB8([255, 255, 255]);
pub const DEFAULT_BACKGROUND: RGB8 = RGB8([0, 0, 0]);

/// An RGB value, using 8 bits for each color channel.
#[cfg_attr(feature = "persistence", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct RGB8(pub [u8; 3]);

/// Handles the graphics state of the `Chip8`.
#[cfg_attr(feature = "persistence", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Copy)]
pub struct GraphicsBuffer {
    #[cfg_attr(feature = "persistence", serde(with = "serde_big_array::BigArray"))]
    vram: [RGB8; PIXEL_COUNT],
    pub foreground_rgb: RGB8,
    pub background_rgb: RGB8,
}

impl Default for GraphicsBuffer {
    fn default() -> Self {
        Self {
            vram: [DEFAULT_BACKGROUND; PIXEL_COUNT],
            foreground_rgb: DEFAULT_FOREGROUND,
            background_rgb: DEFAULT_BACKGROUND,
        }
    }
}

impl GraphicsBuffer {
    /// Create a new empty `GraphicsBuffer`.
    pub fn new() -> Self {
        Self::default()
    }

    /// Draws a byte as a sprite at the given coordinates.
    /// Returns whether or not there was a collision
    pub fn draw_byte(&mut self, x: usize, y: usize, data: u8) -> bool {
        // clipping check
        if y > HEIGHT {
            return false;
        }

        let max_x = (WIDTH as isize - x as isize).clamp(0, 8) as usize;

        let mut collision = false;
        // iterate bits
        for b in 0..max_x {
            let pos = ((WIDTH * y) + x + b) % PIXEL_COUNT;
            let new_pixel_active = (data & (0x80 >> b)) != 0;
            let old_pixel_active = self.vram[pos] == self.foreground_rgb;
            if new_pixel_active && old_pixel_active {
                collision = true;
            }
            let new_pixel_state = new_pixel_active ^ old_pixel_active;
            if new_pixel_state {
                self.vram[pos] = self.foreground_rgb;
            } else {
                self.vram[pos] = self.background_rgb;
            }
        }
        collision
    }

    /// Get the RGB8 pixel buffer representation of this graphics buffer.
    /// The length of the buffer will be `PIXEL_COUNT * COLOR_CHANNEL_COUNT`.
    pub fn as_rgb8(&self) -> [u8; PIXEL_COUNT * 3] {
        let mut data = [0; PIXEL_COUNT * 3];
        // safety: the length of the following iterator should be len(self.vram) * 3, which
        // is equal to the length of `data`.
        self.vram
            .iter()
            .flat_map(|RGB8(color)| color)
            .enumerate()
            .for_each(|(i, x)| {
                data[i] = *x;
            });
        data
    }

    /// Set the foreground color used by the RGB representation of the graphics buffer.
    #[inline]
    pub fn set_foreground_color(&mut self, foreground: RGB8) {
        self.vram.iter_mut().for_each(|color| {
            if *color == self.foreground_rgb {
                *color = foreground;
            }
        });
        self.foreground_rgb = foreground;
    }

    /// Set the background color used by the RGB representation of the graphics buffer.
    #[inline]
    pub fn set_background_color(&mut self, background: RGB8) {
        self.vram.iter_mut().for_each(|color| {
            if *color == self.background_rgb {
                *color = background;
            }
        });
        self.background_rgb = background;
    }

    /// Clear the graphics buffer with the background color.
    #[inline]
    pub fn clear(&mut self) {
        self.vram = [self.background_rgb; PIXEL_COUNT];
    }
}
