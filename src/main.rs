use std::{collections::HashMap, sync::mpsc::TryRecvError};

use mcproto_rs::status;
use network::NetworkCommand;
use server::{InputState, Server};
use settings::Settings;
use tracing_subscriber::{prelude::*, EnvFilter};
use wgpu_app::{utils::persistent_window::PersistentWindowManager, Application};
use winit::{
    dpi::{PhysicalPosition, PhysicalSize},
    event::WindowEvent,
    window::WindowBuilder,
};

pub mod chat;
pub mod entities;
pub mod gui;
pub mod network;
pub mod player;
pub mod resources;
pub mod server;
pub mod settings;
pub mod world;

type WindowManagerType = App;
type WindowManager = PersistentWindowManager<WindowManagerType>;

pub struct App {
    settings: Settings,

    server: Option<Server>,

    pub outstanding_server_pings: HashMap<String, Server>,
    pub server_pings: HashMap<String, status::StatusSpec>,
    // pub icon_handles: HashMap<String, RetainedImage>,
    pub window_manager: PersistentWindowManager<WindowManagerType>,
}

impl App {
    pub fn new() -> Self {
        Self {
            settings: Settings::load()
                .map_err(|e| tracing::error!("Couldn't load settings ({e}), creating new."))
                .unwrap_or_default(),
            server: None,

            outstanding_server_pings: HashMap::new(),
            server_pings: HashMap::new(),

            window_manager: PersistentWindowManager::new(),
        }
    }

    pub const fn settings(&self) -> &Settings {
        &self.settings
    }

    pub fn settings_mut(&mut self) -> &mut Settings {
        &mut self.settings
    }
}

impl Application for App {
    fn init(&mut self, _ctx: &mut wgpu_app::context::Context) {
        tracing::info!("Opening!");
    }

    fn update(&mut self, t: &wgpu_app::Timer, ctx: &mut wgpu_app::context::Context) {
        let delta = t.delta();

        // Server stuff
        if let Some(server) = &mut self.server {
            // Update
            server.update(ctx, delta, &mut self.settings);

            // Mouse handling
            ctx.block_gui_tab_input = server.get_input_state() == InputState::InteractingInfo;
            ctx.block_gui_input = server.should_grab_mouse();

            // TODO - Context grab and hide mouse

            // Disconnect
            match &server.connection {
                server::ConnectionState::Connected => {}
                server::ConnectionState::ClientDisconnected => self.server = None,
                server::ConnectionState::ServerDisconnected(reason) => {
                    self.window_manager
                        .push(gui::disconnect_window(Some(reason.clone())));
                    self.server = None;
                }
            }
        } else {
            // Don't get stuck in the main menu without being able to interact with the UI
            ctx.block_gui_input = false;
            ctx.block_gui_tab_input = false;
        }

        // Outstanding server pings
        self.outstanding_server_pings
            .retain(|k, v| match v.network.recv.try_recv() {
                Ok(NetworkCommand::ReceiveStatus(status)) => {
                    self.server_pings.insert(k.clone(), status);
                    false
                }
                Err(TryRecvError::Disconnected) => false,
                _ => true,
            });
    }

    fn render(
        &mut self,
        t: &wgpu_app::Timer,
        ctx: &mut wgpu_app::context::Context,
    ) -> Result<(), wgpu::SurfaceError> {
        let output = ctx.wgpu_state.surface.get_current_texture()?;

        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder =
            ctx.wgpu_state
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Render Encoder"),
                });

        // *********************** WGPU

        {
            // Clear screen
            let _render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.3,
                            g: 0.6,
                            b: 0.9,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });
        }

        // *********************** Egui
        ctx.egui
            .render(&mut ctx.wgpu_state, &view, &mut encoder, |gui_ctx| {
                gui::render(gui_ctx, self, t);

                // Render windows
                if self.server.as_ref().map_or(true, Server::is_paused) {
                    let mut dummy_manager = WindowManager::new();
                    std::mem::swap(&mut self.window_manager, &mut dummy_manager);
                    dummy_manager.render(self, gui_ctx);
                    std::mem::swap(&mut self.window_manager, &mut dummy_manager);
                }
            });

        // Render
        ctx.wgpu_state.queue.submit([encoder.finish()]);

        output.present();

        Ok(())
    }

    fn close(&mut self, _ctx: &wgpu_app::context::Context) {
        tracing::info!("Closing");

        self.settings
            .save()
            .map_err(|e| tracing::error!("Couldn't save settings ({e})"))
            .ok();
    }

    fn handle_event(
        &mut self,
        _ctx: &mut wgpu_app::context::Context,
        event: &winit::event::Event<()>,
    ) {
        match event {
            winit::event::Event::WindowEvent {
                window_id: _,
                event: WindowEvent::Resized(new_size),
            } => {
                self.settings.window_size = [new_size.width, new_size.height];
            }
            winit::event::Event::WindowEvent {
                window_id: _,
                event: WindowEvent::Moved(new_pos),
            } => {
                self.settings.window_pos = Some([new_pos.x, new_pos.y]);
            }
            _ => {}
        }
    }
}

fn main() {
    init_tracing();

    let app = App::new();

    let &[w, h] = &app.settings.window_size;
    let mut wb = WindowBuilder::new()
        .with_title("Mink Raft :3")
        .with_inner_size(PhysicalSize::new(w, h))
        .with_min_inner_size(PhysicalSize::new(200, 200))
        .with_resizable(true);

    if let Some(&[x, y]) = app.settings.window_pos.as_ref() {
        wb = wb.with_position(PhysicalPosition::new(x, y));
    }

    wgpu_app::run(app, wb);
}

pub fn init_tracing() {
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "info");
    }

    let subscriber = tracing_subscriber::registry().with(
        tracing_subscriber::fmt::layer()
            .with_writer(std::io::stderr)
            .with_filter(EnvFilter::from_default_env()),
    );

    subscriber.init();
}
