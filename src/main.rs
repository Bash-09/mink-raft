use wgpu_app::Application;
use winit::{dpi::PhysicalSize, window::WindowBuilder};

pub mod chat;
pub mod gui;
pub mod server;

struct App {}

impl App {
    pub fn new() -> Self {
        Self {}
    }
}

impl Application for App {
    fn init(&mut self, ctx: &mut wgpu_app::context::Context) {
        println!("Opening!");
    }

    fn update(&mut self, t: &wgpu_app::Timer, ctx: &mut wgpu_app::context::Context) {}

    fn render(
        &self,
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
                gui::fps_counter(gui_ctx, t.fps(), t.delta());

                egui::Window::new("Memory").show(gui_ctx, |ui| {
                    ui.heading("Cum");
                });
            });

        // Render
        ctx.wgpu_state.queue.submit([encoder.finish()]);

        output.present();

        Ok(())
    }

    fn close(&mut self, ctx: &wgpu_app::context::Context) {
        println!("Closing");
    }

    fn handle_event(
        &mut self,
        ctx: &mut wgpu_app::context::Context,
        event: &winit::event::Event<()>,
    ) {
    }
}

fn main() {
    let wb = WindowBuilder::new()
        .with_title("Mink Raft :3")
        .with_inner_size(PhysicalSize::new(1200, 700))
        .with_resizable(true);

    let app = App::new();

    wgpu_app::run(app, wb);
}
