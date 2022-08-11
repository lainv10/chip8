/// A response for a requested key request by the processor.
/// Contains the pressed key's key code and the register
/// the processor should store it in.
#[cfg_attr(feature = "persistence", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Copy)]
pub struct KeyRequestResponse {
    pub key_code: u8,
    pub register: usize,
}

/// Input system for the `Chip8`. This keeps track of the pressed state of all 16 keys,
/// as well as any key press requests from programs.
#[cfg_attr(feature = "persistence", derive(serde::Serialize, serde::Deserialize))]
#[derive(Default)]
pub struct Input {
    state: [bool; 16],
    waiting: bool,
    request_reg: usize,
    request_response: Option<KeyRequestResponse>,
}

impl Input {
    /// Create a new [`Input`] instance.
    pub fn new() -> Self {
        Self::default()
    }

    /// Update the input state of the given key code.
    pub fn update(&mut self, key_code: u8, pressed: bool) {
        self.state[usize::from(key_code)] = pressed;
        if pressed && self.waiting {
            self.waiting = false;
            self.request_response = Some(KeyRequestResponse {
                key_code,
                register: self.request_reg,
            });
        }
    }

    /// Request a single key press from the user.
    pub fn request_key_press(&mut self, register: usize) {
        self.waiting = true;
        self.request_reg = register;
    }

    /// Get the input request response. This will be `None` if
    /// no key event was requested, or if the key event was
    /// already consumed.
    pub fn request_response(&mut self) -> Option<KeyRequestResponse> {
        self.request_response.take()
    }

    /// Returns whether the system is currently
    /// waiting for user input.
    pub fn waiting(&self) -> bool {
        self.waiting
    }

    /// Returns whether the given key is currently pressed.
    pub fn is_key_pressed(&self, key_code: u8) -> bool {
        self.state[usize::from(key_code)]
    }
}
