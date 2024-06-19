use crate::chat::Chat;

pub struct Server {
    world_time: i64,

    chat: Chat,
}

impl Server {
    pub fn new() -> Self {
        Self {
            world_time: 0,
            chat: Chat::new(),
        }
    }

    pub const fn get_world_time(&self) -> i64 {
        self.world_time
    }

    pub const fn get_chat(&self) -> &Chat {
        &self.chat
    }

    pub fn get_chat_mut(&mut self) -> &mut Chat {
        &mut self.chat
    }
}
