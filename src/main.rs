use std::collections::HashMap;

use mcproto_rs::status;
use server::Server;
use settings::Settings;
use tracing_subscriber::{prelude::*, EnvFilter};
use wgpu_app::{utils::persistent_window::PersistentWindowManager, Application};
use winit::{dpi::PhysicalSize, window::WindowBuilder};

pub mod chat;
pub mod entities;
pub mod gui;
pub mod network;
pub mod player;
pub mod resources;
pub mod server;
pub mod settings;
pub mod world;

pub type WindowManagerType = App;
pub type WindowManager = PersistentWindowManager<WindowManagerType>;

struct App {
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
        _event: &winit::event::Event<()>,
    ) {
    }
}

fn main() {
    init_tracing();

    let wb = WindowBuilder::new()
        .with_title("Mink Raft :3")
        .with_inner_size(PhysicalSize::new(1200, 700))
        .with_resizable(true);

    let app = App::new();

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
