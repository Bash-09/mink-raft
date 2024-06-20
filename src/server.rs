use std::{collections::HashMap, f64::consts::PI, ops::AddAssign};

use glam::{DVec3, IVec2};
use mcproto_rs::{
    types::{self, EntityLocation, VarInt},
    uuid::UUID4,
    v1_16_3::{
        ClientStatusAction, Difficulty, GameMode, PlayClientChatMessageSpec,
        PlayClientPlayerPositionAndRotationSpec, PlayClientSettingsSpec, PlayClientStatusSpec,
        PlayTeleportConfirmSpec, PlayerInfoAction,
    },
};
use wgpu_app::{context::Context, Timer};
use winit::keyboard::KeyCode;

use crate::{
    gui::{chat_windows, info_windows, pause_windows},
    network::{encode, NetworkChannel, NetworkCommand, PacketType},
    // resources::PLAYER_INDEX,
    settings::Settings,
    world::chunks::Chunk,
    WindowManager,
};

use self::remote_player::RemotePlayer;

use super::{chat::Chat, entities::Entity, player::Player, world::World};

pub mod remote_player;

pub struct Server {
    network_destination: String,
    pub network: NetworkChannel,

    input_state: InputState,

    world_time: i64,
    day_time: i64,

    position_update_timer: Timer,

    player: Player,
    chat: Chat,

    world: World,

    entities: HashMap<i32, Entity>,
    players: HashMap<UUID4, RemotePlayer>,

    difficulty: Difficulty,
    difficulty_locked: bool,

    pub connection: ConnectionState,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConnectionState {
    Connected,
    ClientDisconnected,
    ServerDisconnected(String),
}

/// The input state of the player.
/// `Playing` - Normal fps input where the mouse and keyboard control the player
/// `Paused` - Paused menu is visible, mouse and keyboard are visible and interact with ui
/// `ShowingInfo` - Similar to Playing but also showing a handful of debug and other useful info,
/// clicking will transition to `InteractingInfo`
/// `InteractingInfo` - Debug and other useful info is visible, mouse is visible and can interact
/// with the info windows
/// `ChatOpen` - Chat is visible and interactable, mouse is visible and can scroll through the chat
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum InputState {
    Playing,
    Paused,
    ShowingInfo,
    InteractingInfo,
    ChatOpen,
}

impl Server {
    #[must_use]
    pub fn new(network_destination: String, network: NetworkChannel) -> Self {
        Self {
            network_destination,
            network,

            input_state: InputState::Playing,

            world_time: 0,
            day_time: 0,

            player: Player::new(),
            chat: Chat::new(),

            world: World::new(),

            position_update_timer: Timer::new_with_period(0.05),

            entities: HashMap::new(),
            players: HashMap::new(),

            difficulty: Difficulty::Easy,
            difficulty_locked: false,

            connection: ConnectionState::Connected,
        }
    }

    #[must_use]
    pub fn get_network_destination(&self) -> &str {
        &self.network_destination
    }

    #[must_use]
    pub fn get_input_state(&self) -> InputState {
        self.input_state
    }

    #[must_use]
    pub fn get_world_time(&self) -> i64 {
        self.world_time
    }

    #[must_use]
    pub fn get_day_time(&self) -> i64 {
        self.day_time
    }

    #[must_use]
    pub fn get_player(&self) -> &Player {
        &self.player
    }

    #[must_use]
    pub fn get_chat(&self) -> &Chat {
        &self.chat
    }

    pub fn get_chat_mut(&mut self) -> &mut Chat {
        &mut self.chat
    }

    #[must_use]
    pub fn get_world(&self) -> &World {
        &self.world
    }

    #[must_use]
    pub fn get_entities(&self) -> &HashMap<i32, Entity> {
        &self.entities
    }

    #[must_use]
    pub fn get_difficulty(&self) -> Difficulty {
        self.difficulty.clone()
    }

    #[must_use]
    pub fn is_difficulty_locked(&self) -> bool {
        self.difficulty_locked
    }

    #[must_use]
    pub fn is_paused(&self) -> bool {
        self.input_state == InputState::Paused
    }

    pub fn set_input_state(&mut self, state: InputState) {
        self.input_state = state;
    }

    pub fn join_game(&mut self, player_id: i32) {
        self.player.id = player_id;
    }

    #[must_use]
    pub fn get_players(&self) -> &HashMap<UUID4, RemotePlayer> {
        &self.players
    }

    /// Generates a sky colour based on a provided base colour and the current time of day on the
    /// server
    #[must_use]
    pub fn get_sky_colour(&self, col: &[f64; 3]) -> DVec3 {
        const LIGHTEST: i64 = 9_000;
        let lerp = (((self.day_time - LIGHTEST) as f64 / 24_000.0) * PI * 2.0).cos() / 2.0 + 0.5;
        let dark = DVec3::new(0.001, 0.002, 0.005);
        let light = DVec3::from(*col);
        dark.lerp(light, lerp)
    }

    /// Attempts to send a packet over the provided (possible) network channel
    pub fn send_packet(&self, packet: Vec<u8>) {
        if let Err(e) = self.network.send.send(NetworkCommand::SendPacket(packet)) {
            tracing::error!("Failed to communicate with network commander: {:?}", e);
            panic!("Disconnected");
        }
    }

    /// Attempts to send a packet over the provided (possible) network channel
    pub fn send_command(&self, command: NetworkCommand) {
        if let Err(e) = self.network.send.send(command) {
            tracing::error!("Failed to communicate with network commander: {:?}", e);
            panic!("Disconnected");
        }
    }

    pub fn should_grab_mouse(&self) -> bool {
        match self.input_state {
            InputState::Playing => true,
            InputState::Paused => false,
            InputState::ShowingInfo => true,
            InputState::InteractingInfo => false,
            InputState::ChatOpen => false,
        }
    }

    pub fn render(&mut self, gui_ctx: &egui::Context, windows: &mut WindowManager) {
        if self.input_state != InputState::ChatOpen {
            chat_windows::render_inactive(self, gui_ctx);
        }

        match self.input_state {
            InputState::Playing => {}
            InputState::Paused => match pause_windows::render(gui_ctx, windows) {
                pause_windows::PauseAction::Disconnect => self.disconnect(),
                pause_windows::PauseAction::Unpause => self.set_input_state(InputState::Playing),
                pause_windows::PauseAction::Nothing => {}
            },
            InputState::ShowingInfo | InputState::InteractingInfo => {
                info_windows::render(gui_ctx, self)
            }
            InputState::ChatOpen => chat_windows::render_active(self, gui_ctx),
        }
    }

    pub fn update(&mut self, ctx: &Context, delta: f64, settings: &mut Settings) {
        // self.world.generate_meshes(&ctx.dis, true);

        // Update entities
        for ent in self.entities.values_mut() {
            ent.update(delta);
        }

        // Handle input
        match self.input_state {
            InputState::Playing => self.handle_playing_state(ctx, delta, settings),
            InputState::Paused => self.handle_paused_state(ctx, delta, settings),
            InputState::ShowingInfo => self.handle_show_info_state(ctx, delta, settings),
            InputState::InteractingInfo => self.handle_interact_info_state(ctx, delta, settings),
            InputState::ChatOpen => self.handle_chat_open_state(ctx, delta, settings),
        }

        // Handle messages from the NetworkManager
        loop {
            match self.network.recv.try_recv() {
                Ok(comm) => self.handle_message(comm, ctx),
                Err(e) => match e {
                    std::sync::mpsc::TryRecvError::Empty => break,
                    std::sync::mpsc::TryRecvError::Disconnected => {
                        tracing::error!(
                            "Could not communicate with server. Assuming disconnected."
                        );

                        if self.connection == ConnectionState::Connected {
                            self.connection = ConnectionState::ServerDisconnected(String::from("Server forced disconnect. (You were probably sending too many connection requests)"));
                        }
                        return;
                    }
                },
            }
        }

        // Send player position updates
        if self.position_update_timer.go().is_some() && self.player.id != 0 {
            self.send_packet(encode(PacketType::PlayClientPlayerPositionAndRotation(
                PlayClientPlayerPositionAndRotationSpec {
                    feet_location: EntityLocation {
                        position: types::Vec3 {
                            x: self.get_player().get_position().x,
                            y: self.get_player().get_position().y,
                            z: self.get_player().get_position().z,
                        },
                        rotation: types::EntityRotation {
                            yaw: self.get_player().get_orientation().get_yaw() as f32,
                            pitch: self.get_player().get_orientation().get_pitch() as f32,
                        },
                    },
                    on_ground: true,
                },
            )));
        }
    }

    fn handle_playing_state(&mut self, ctx: &Context, delta: f64, settings: &mut Settings) {
        if ctx.keyboard.pressed_this_frame(KeyCode::Escape) {
            self.input_state = InputState::Paused;
        } else if ctx.keyboard.pressed_this_frame(KeyCode::KeyT) {
            self.input_state = InputState::ChatOpen;
        } else if ctx.keyboard.pressed_this_frame(KeyCode::Slash) {
            self.input_state = InputState::ChatOpen;
            self.chat.set_current_message(String::from("/"));
        } else if ctx.keyboard.pressed_this_frame(KeyCode::Tab) {
            self.input_state = InputState::ShowingInfo;
        }

        self.handle_keyboard_movement(ctx, delta, settings);
        self.handle_mouse_movement(ctx, delta, settings);
    }

    fn handle_paused_state(&mut self, ctx: &Context, _delta: f64, _settings: &mut Settings) {
        if ctx.keyboard.pressed_this_frame(KeyCode::Escape) {
            self.input_state = InputState::Playing;
        }
    }

    fn handle_show_info_state(&mut self, ctx: &Context, delta: f64, settings: &mut Settings) {
        if ctx.keyboard.pressed_this_frame(KeyCode::Escape) {
            self.input_state = InputState::Paused;
        } else if ctx.mouse.pressed_this_frame(0) {
            self.input_state = InputState::InteractingInfo;
        } else if ctx.keyboard.released_this_frame(KeyCode::Tab) {
            self.input_state = InputState::Playing;
        }

        self.handle_keyboard_movement(ctx, delta, settings);
        self.handle_mouse_movement(ctx, delta, settings);
    }

    fn handle_interact_info_state(&mut self, ctx: &Context, delta: f64, settings: &mut Settings) {
        if ctx.keyboard.pressed_this_frame(KeyCode::Escape) {
            self.input_state = InputState::Paused;
        } else if ctx.keyboard.released_this_frame(KeyCode::Tab) {
            self.input_state = InputState::Playing;
        }

        self.handle_keyboard_movement(ctx, delta, settings);
    }

    fn handle_chat_open_state(&mut self, ctx: &Context, _delta: f64, _settings: &mut Settings) {
        if ctx.keyboard.pressed_this_frame(KeyCode::Escape) {
            self.input_state = InputState::Playing;
        } else if ctx.keyboard.pressed_this_frame(KeyCode::Enter) {
            let text = self.chat.get_current_message_and_clear();
            if !text.is_empty() {
                self.send_packet(encode(PacketType::PlayClientChatMessage(
                    PlayClientChatMessageSpec { message: text },
                )));
            }
            self.input_state = InputState::Playing;
        }
    }

    pub fn handle_mouse_movement(&mut self, ctx: &Context, _delta: f64, settings: &mut Settings) {
        let off = ctx.mouse.get_delta();
        self.player.get_orientation_mut().rotate(
            off.0 as f64 * 0.05 * settings.mouse_sensitivity,
            off.1 as f64 * 0.05 * settings.mouse_sensitivity,
        );
    }

    pub fn handle_keyboard_movement(
        &mut self,
        ctx: &Context,
        delta: f64,
        _settings: &mut Settings,
    ) {
        let vel = 14.0 * delta;

        if ctx.keyboard.is_pressed(KeyCode::KeyW) {
            let mut dir = self.player.get_orientation().get_look_vector();
            dir.y = 0.0;
            dir = dir.normalize();
            dir *= vel;
            self.player.get_position_mut().add_assign(dir);
        }

        if ctx.keyboard.is_pressed(KeyCode::KeyS) {
            let mut dir = self.player.get_orientation().get_look_vector();
            dir.y = 0.0;
            dir = dir.normalize();
            dir *= -vel;
            self.player.get_position_mut().add_assign(dir);
        }

        if ctx.keyboard.is_pressed(KeyCode::KeyA) {
            let mut dir = self.player.get_orientation().get_look_vector();
            dir.y = 0.0;
            dir = dir.normalize();
            dir *= -vel;
            dir.y = dir.x; // Just using this value as temp to swap x and z
            dir.x = -dir.z;
            dir.z = dir.y;
            dir.y = 0.0;
            self.player.get_position_mut().add_assign(dir);
        }

        if ctx.keyboard.is_pressed(KeyCode::KeyD) {
            let mut dir = self.player.get_orientation().get_look_vector();
            dir.y = 0.0;
            dir = dir.normalize();
            dir *= vel;
            dir.y = dir.x; // Just using this value as temp to swap x and z
            dir.x = -dir.z;
            dir.z = dir.y;
            dir.y = 0.0;
            self.player.get_position_mut().add_assign(dir);
        }

        if ctx.keyboard.is_pressed(KeyCode::Space) {
            self.player
                .get_position_mut()
                .add_assign(DVec3::new(0.0, vel, 0.0));
        }

        if ctx.keyboard.is_pressed(KeyCode::ShiftLeft) {
            self.player
                .get_position_mut()
                .add_assign(DVec3::new(0.0, -vel, 0.0));
        }
    }

    pub fn disconnect(&mut self) {
        tracing::info!("Disconnecting from server.");
        self.network
            .send
            .send(NetworkCommand::Disconnect)
            .expect("Failed to send message to network thread.");
        self.connection = ConnectionState::ClientDisconnected;
    }

    /// Handles a message from the `NetworkManager`
    #[allow(clippy::too_many_lines, clippy::cognitive_complexity)]
    fn handle_message(&mut self, comm: NetworkCommand, _ctx: &Context) {
        #[allow(clippy::enum_glob_use)]
        use NetworkCommand::*;

        match comm {
            // Handles any incoming packets
            ReceivePacket(packet) => {
                match packet {
                    PacketType::PlayServerDifficulty(pack) => {
                        self.difficulty = pack.difficulty;
                        self.difficulty_locked = pack.locked;
                        tracing::info!("Changed difficulty: {}", pack.locked);
                    }

                    PacketType::PlayTimeUpdate(pack) => {
                        self.world_time = pack.world_age;
                        self.day_time = pack.time_of_day;
                    }

                    PacketType::PlayUpdatehealth(pack) => {
                        self.player.health = pack.health;
                        self.player.food = pack.food.0;
                        self.player.saturation = pack.saturation;
                    }

                    PacketType::PlayDisconnect(pack) => {
                        let disconnect_reason = pack.reason.to_traditional();
                        tracing::info!("Disconnected from server: {disconnect_reason:?}");
                        self.connection = ConnectionState::ServerDisconnected(
                            disconnect_reason.unwrap_or_else(|| String::from("No reason given")),
                        );
                    }

                    PacketType::LoginSuccess(_) => {
                        tracing::info!("Successfully Logged in!");
                    }

                    PacketType::LoginDisconnect(pack) => {
                        tracing::info!("Disconnected during login");
                        self.connection = ConnectionState::ServerDisconnected(
                            pack.message
                                .to_traditional()
                                .unwrap_or_else(|| String::from("No reason given")),
                        );
                    }

                    PacketType::PlayJoinGame(id) => {
                        self.join_game(id.entity_id);
                        self.send_packet(encode(PacketType::PlayClientSettings(
                            PlayClientSettingsSpec {
                                locale: self.player.locale.clone(),
                                view_distance: (self.player.view_distance),
                                chat_mode: self.player.chat_mode.clone(),
                                chat_colors: (false),
                                displayed_skin_parts: self.player.displayed_skin_parts,
                                main_hand: self.player.main_hand.clone(),
                            },
                        )));
                        self.send_packet(encode(PacketType::PlayClientStatus(
                            PlayClientStatusSpec {
                                action: ClientStatusAction::PerformRespawn,
                            },
                        )));
                    }

                    PacketType::PlaySpawnPlayer(pack) => {
                        self.entities.insert(
                            pack.entity_id.0,
                            Entity::new_with_values(
                                pack.entity_id.0,
                                pack.uuid,
                                // PLAYER_INDEX as u32,
                                0,
                                0,
                                pack.location.position.x,
                                pack.location.position.y,
                                pack.location.position.z,
                                pack.location.rotation.yaw.value as f64 / 255.0,
                                pack.location.rotation.pitch.value as f64 / 255.0,
                                pack.location.rotation.pitch.value as f64 / 255.0,
                                0.0,
                                0.0,
                                0.0,
                            ),
                        );
                    }

                    PacketType::PlaySpawnLivingEntity(pack) => {
                        self.entities.insert(
                            pack.entity_id.0,
                            Entity::new_with_values(
                                pack.entity_id.0,
                                pack.entity_uuid,
                                pack.entity_type.0 as u32,
                                0,
                                pack.location.position.x,
                                pack.location.position.y,
                                pack.location.position.z,
                                pack.location.rotation.yaw.value as f64 / 255.0,
                                pack.location.rotation.pitch.value as f64 / 255.0,
                                pack.head_pitch.value as f64 / 255.0,
                                pack.velocity.x as f64 / 400.0,
                                pack.velocity.y as f64 / 400.0,
                                pack.velocity.z as f64 / 400.0,
                            ),
                        );
                    }

                    PacketType::PlaySpawnEntity(pack) => {
                        self.entities.insert(
                            pack.entity_id.0,
                            Entity::new_with_values(
                                pack.entity_id.0,
                                pack.object_uuid,
                                pack.entity_type.0 as u32,
                                pack.data,
                                pack.position.x as f64,
                                pack.position.y as f64,
                                pack.position.z as f64,
                                pack.yaw.value as f64 / 255.0,
                                pack.pitch.value as f64 / 255.0,
                                0.0,
                                pack.velocity.x as f64 / 400.0,
                                pack.velocity.y as f64 / 400.0,
                                pack.velocity.z as f64 / 400.0,
                            ),
                        );
                    }

                    PacketType::PlayDestroyEntities(pack) => {
                        for eid in pack.entity_ids.iter() {
                            self.entities.remove(&eid.0);
                        }
                    }

                    PacketType::PlayEntityPosition(pack) => {
                        if let Some(ent) = self.entities.get_mut(&pack.entity_id.0) {
                            let new_pos = ent.last_pos
                                + DVec3::new(
                                    (pack.delta.x as f64) / 4096.0,
                                    (pack.delta.y as f64) / 4096.0,
                                    (pack.delta.z as f64) / 4096.0,
                                );
                            ent.pos = new_pos;
                            ent.last_pos = new_pos;
                        }
                    }

                    PacketType::PlayEntityPositionAndRotation(pack) => {
                        if let Some(ent) = self.entities.get_mut(&pack.entity_id.0) {
                            let new_pos = ent.last_pos
                                + DVec3::new(
                                    (pack.delta.position.x as f64) / 4096.0,
                                    (pack.delta.position.y as f64) / 4096.0,
                                    (pack.delta.position.z as f64) / 4096.0,
                                );
                            ent.pos = new_pos;
                            ent.last_pos = new_pos;
                            ent.ori.set(
                                pack.delta.rotation.yaw.value as f64 / 256.0,
                                pack.delta.rotation.pitch.value as f64 / 256.0,
                            );
                            ent.on_ground = pack.on_ground;
                        }
                    }

                    PacketType::PlayEntityRotation(pack) => {
                        if let Some(ent) = self.entities.get_mut(&pack.entity_id.0) {
                            ent.ori.set(
                                pack.rotation.yaw.value as f64 / 256.0,
                                pack.rotation.pitch.value as f64 / 256.0,
                            );
                            ent.on_ground = pack.on_ground;
                        }
                    }

                    PacketType::PlayEntityHeadLook(pack) => {
                        if let Some(ent) = self.entities.get_mut(&pack.entity_id.0) {
                            ent.ori_head.set(
                                f64::from(pack.head_yaw.value) / 256.0,
                                ent.ori_head.get_pitch(),
                            );
                        }
                    }

                    PacketType::PlayEntityVelocity(pack) => {
                        if let Some(ent) = self.entities.get_mut(&pack.entity_id.0) {
                            ent.vel = DVec3::new(
                                f64::from(pack.velocity.x) / 400.0,
                                f64::from(pack.velocity.y) / 400.0,
                                f64::from(pack.velocity.z) / 400.0,
                            );
                        }
                    }

                    PacketType::PlayEntityTeleport(pack) => {
                        if let Some(ent) = self.entities.get_mut(&pack.entity_id.0) {
                            ent.pos = DVec3::new(
                                pack.location.position.x,
                                pack.location.position.y,
                                pack.location.position.z,
                            );
                            ent.ori.set(
                                f64::from(pack.location.rotation.yaw.value) / 256.0,
                                f64::from(pack.location.rotation.pitch.value) / 256.0,
                            );
                            ent.on_ground = pack.on_ground;
                        }
                    }

                    PacketType::PlayServerPlayerPositionAndLook(pack) => {
                        tracing::debug!("Player position updated!");

                        self.player.set_position(DVec3::new(
                            pack.location.position.x,
                            pack.location.position.y,
                            pack.location.position.z,
                        ));
                        self.player.get_orientation_mut().set(
                            f64::from(pack.location.rotation.yaw),
                            f64::from(pack.location.rotation.pitch),
                        );

                        self.send_packet(encode(PacketType::PlayTeleportConfirm(
                            PlayTeleportConfirmSpec {
                                teleport_id: pack.teleport_id,
                            },
                        )));

                        let x = self.player.get_position().x;
                        let y = self.player.get_position().y;
                        let z = self.player.get_position().z;
                        self.send_packet(encode(PacketType::PlayClientPlayerPositionAndRotation(
                            PlayClientPlayerPositionAndRotationSpec {
                                on_ground: (true),
                                feet_location: EntityLocation {
                                    position: types::Vec3 { x, y, z },
                                    rotation: pack.location.rotation,
                                },
                            },
                        )));
                    }

                    PacketType::PlayServerChatMessage(chat) => {
                        self.chat.add_message(chat, self.world_time);
                    }

                    PacketType::PlayChunkData(cd) => {
                        self.world.insert_chunk(Chunk::new(&cd.data));
                    }

                    PacketType::PlayUnloadChunk(pack) => {
                        self.world
                            .get_chunks_mut()
                            .remove(&IVec2::new(pack.position.x, pack.position.z));
                    }

                    PacketType::PlayBlockChange(pack) => {
                        self.world.handle_block_change(pack);
                    }

                    PacketType::PlayMultiBlockChange(pack) => {
                        self.world.handle_multi_block_change(pack);
                    }

                    PacketType::PlayPlayerInfo(pack) => {
                        use mcproto_rs::v1_16_3::PlayerInfoActionList;
                        match pack.actions {
                            PlayerInfoActionList::Add(players) => {
                                for player in players.iter() {
                                    self.players.insert(
                                        player.uuid,
                                        RemotePlayer {
                                            uuid: player.uuid,
                                            name: player.action.name.clone(),
                                            gamemode: player.action.game_mode.clone(),
                                            ping: player.action.ping_ms.0,
                                            display_name: player
                                                .action
                                                .display_name
                                                .clone()
                                                .map(|dn| dn.to_traditional())
                                                .unwrap_or(None),
                                        },
                                    );
                                }
                            }
                            PlayerInfoActionList::UpdateGameMode(players) => {
                                let players: Vec<PlayerInfoAction<GameMode>> = From::from(players);
                                for player in players {
                                    if let Some(p) = self.players.get_mut(&player.uuid) {
                                        p.gamemode = player.action;
                                    }
                                }
                            }
                            PlayerInfoActionList::UpdateLatency(players) => {
                                let players: Vec<PlayerInfoAction<VarInt>> = From::from(players);
                                for player in players {
                                    if let Some(p) = self.players.get_mut(&player.uuid) {
                                        p.ping = player.action.into();
                                    }
                                }
                            }
                            PlayerInfoActionList::UpdateDisplayName(players) => {
                                for player in players.iter() {
                                    if let Some(p) = self.players.get_mut(&player.uuid) {
                                        p.display_name = player.action.clone().map(|chat| {
                                            chat.to_traditional().unwrap_or_else(|| {
                                                "Failed to parse name".to_string()
                                            })
                                        });
                                    }
                                }
                            }
                            PlayerInfoActionList::Remove(players) => {
                                for player in players.iter() {
                                    self.players.remove(player);
                                }
                            }
                        }
                    }

                    // Currently ignoring these packets
                    PacketType::PlayEntityMetadata(_)
                    | PacketType::PlayEntityProperties(_)
                    | PacketType::PlayEntityStatus(_)
                    | PacketType::PlayEntityAnimation(_) => {}

                    // Packets that have been forwarded but not handled properly
                    _ => {
                        tracing::debug!("Got Packet: {:?}", packet);
                    }
                }
            }

            // What do with these messages ay??
            _ => {
                tracing::debug!("Unhandled message: {:?}", comm);
            }
        }
    }
}
