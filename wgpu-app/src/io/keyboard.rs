use egui_winit::winit::event::{ElementState, Event, WindowEvent};
use winit::keyboard::PhysicalKey;

use std::collections::HashMap;

pub struct Keyboard {
    keys: HashMap<PhysicalKey, bool>,
    this_frame: HashMap<PhysicalKey, bool>,
}

impl Keyboard {
    #[must_use]
    pub fn new() -> Self {
        Self {
            keys: HashMap::new(),
            this_frame: HashMap::new(),
        }
    }

    fn press(&mut self, key: PhysicalKey) {
        self.keys.insert(key, true);
        self.this_frame.insert(key, true);
    }

    fn release(&mut self, key: PhysicalKey) {
        self.keys.insert(key, false);
        self.this_frame.insert(key, true);
    }

    /// This function is called automatically in the application loop, you shouldn't be calling this yourself.
    pub fn handle_event(&mut self, event: &Event<()>) {
        if let Event::WindowEvent {
            window_id: _,
            event: WindowEvent::KeyboardInput { event, .. },
        } = event
        {
            if event.state == ElementState::Pressed {
                self.press(event.physical_key);
            } else {
                self.release(event.physical_key);
            }
        }
    }

    /// Returns if this key was pressed down on this frame
    #[must_use]
    pub fn pressed_this_frame(&self, key: &PhysicalKey) -> bool {
        match self.keys.get(key) {
            None | Some(false) => false,
            Some(true) => match self.this_frame.get(key) {
                None | Some(false) => false,
                Some(true) => true,
            },
        }
    }

    /// Returns if this key was released on this frame
    #[must_use]
    pub fn released_this_frame(&self, key: &PhysicalKey) -> bool {
        match self.keys.get(key) {
            Some(true) => false,
            None | Some(false) => match self.this_frame.get(key) {
                None | Some(false) => false,
                Some(true) => true,
            },
        }
    }

    /// Returns if the key is currently held down
    #[must_use]
    pub fn is_pressed(&self, key: &PhysicalKey) -> bool {
        match self.keys.get(key) {
            None | Some(false) => false,
            Some(true) => true,
        }
    }

    /// Resets the Keyboard for the next frame, this function is called automatically so you shouldn't need to call this function yourself.
    pub fn next_frame(&mut self) {
        self.this_frame.clear();
    }
}

impl Default for Keyboard {
    fn default() -> Self {
        Self::new()
    }
}
