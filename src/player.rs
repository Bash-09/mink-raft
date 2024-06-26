use glam::DVec3;
use mcproto_rs::v1_16_3::{ClientChatMode, ClientDisplayedSkinParts, ClientMainHand};

use super::entities::components::Orientation;

pub struct Player {
    pub id: i32,

    position: DVec3,
    orientation: Orientation,

    pub health: f32,
    pub food: i32,
    pub saturation: f32,

    // Client Settings
    pub locale: String,
    pub view_distance: i8,
    pub chat_mode: ClientChatMode, // 0 - Enabled, 1 - Commands only, 2 - Hidden
    pub displayed_skin_parts: ClientDisplayedSkinParts, // Bitmask - https://wiki.vg/Protocol#Client_Settings
    pub main_hand: ClientMainHand,                      // 0 - Left, 1 - Right
    pub disable_text_filtering: bool,                   // idek what this does
}

impl Player {
    pub fn new() -> Player {
        Player {
            id: 0,

            position: DVec3::new(0.0, 0.0, 0.0),
            orientation: Orientation::new_with_values(0.0, 0.0, -89.0, 89.0),

            health: 20.0,
            food: 20,
            saturation: 5.0,

            locale: String::from("en_GB"),
            view_distance: 8,
            chat_mode: ClientChatMode::Enabled,
            displayed_skin_parts: ClientDisplayedSkinParts::default(),
            main_hand: ClientMainHand::Left,
            disable_text_filtering: true,
        }
    }

    pub fn set_position(&mut self, pos: DVec3) {
        self.position = pos;
    }
    pub fn get_position(&self) -> &DVec3 {
        &self.position
    }

    pub fn set_orientation(&mut self, ori: Orientation) {
        self.orientation = ori;
    }
    pub fn get_orientation(&self) -> &Orientation {
        &self.orientation
    }

    pub fn get_position_mut(&mut self) -> &mut DVec3 {
        &mut self.position
    }
    pub fn get_orientation_mut(&mut self) -> &mut Orientation {
        &mut self.orientation
    }
}
