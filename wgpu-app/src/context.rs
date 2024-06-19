use egui::ViewportId;
use egui_wgpu::ScreenDescriptor;
use egui_winit::EventResponse;
use wgpu::{CommandEncoder, TextureFormat, TextureView};
use winit::{
    dpi::PhysicalSize,
    event::{Event, WindowEvent},
    event_loop::EventLoopWindowTarget,
    keyboard::{KeyCode, PhysicalKey},
    window::Window,
};

use crate::io::{keyboard::Keyboard, mouse::Mouse};

/// `Context` stores some useful things you might want to use in your app, including input from a Keyboard and Mouse,
/// everything you need to render using Wgpu and an `EguiManager` for all your gui needs!
pub struct Context<'a> {
    pub wgpu_state: WgpuState<'a>,
    pub egui: EguiManager,

    pub mouse: Mouse,
    pub keyboard: Keyboard,
    /// If true, Egui will not process new window events
    pub block_gui_input: bool,
    /// If true, Egui will not receive keyboard inputs for the tab key.
    pub block_gui_tab_input: bool,
}

/// Convenience struct to manage the required state to use Egui
pub struct EguiManager {
    renderer: egui_wgpu::Renderer,
    state: egui_winit::State,
}

/// Convenience struct holding everything you need to get rendering with Wgpu
pub struct WgpuState<'a> {
    pub surface: wgpu::Surface<'a>,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub config: wgpu::SurfaceConfiguration,
    pub size: winit::dpi::PhysicalSize<u32>,
    pub window: &'a Window,
}

impl<'a> WgpuState<'a> {
    /// Reconfigure the Wgpu surface for the given size
    pub fn resize(&mut self, size: PhysicalSize<u32>) {
        if size.width < 16 || size.height < 16 {
            return;
        }
        self.config.width = size.width;
        self.config.height = size.height;
        self.surface.configure(&self.device, &self.config);
        self.size = size;
    }
}

impl<'a> Context<'a> {
    pub fn new(wgpu_state: WgpuState, egui: EguiManager) -> Context {
        Context {
            wgpu_state,
            egui,

            mouse: Mouse::new(),
            keyboard: Keyboard::new(),
            block_gui_input: false,
            block_gui_tab_input: false,
        }
    }

    /// This function is automatically called in the application loop, you shouldn't need to call it yourself
    pub fn handle_event(&mut self, event: &Event<()>) {
        self.keyboard.handle_event(event);
        self.mouse.handle_event(event);

        if let winit::event::Event::WindowEvent {
            window_id: _,
            event,
        } = event
        {
            if let winit::event::WindowEvent::Resized(new_size) = event {
                self.wgpu_state.resize(*new_size);
                let _ = self.egui.on_event(self.wgpu_state.window, event);
                return;
            }

            if self.block_gui_input {
                return;
            }

            if self.block_gui_tab_input {
                if let winit::event::WindowEvent::KeyboardInput {
                    event:
                        winit::event::KeyEvent {
                            physical_key: PhysicalKey::Code(KeyCode::Tab),
                            ..
                        },
                    ..
                } = event
                {
                } else {
                    let _ = self.egui.on_event(self.wgpu_state.window, event);
                }
            } else {
                let _ = self.egui.on_event(self.wgpu_state.window, event);
            }
        }

        if let winit::event::Event::DeviceEvent {
            device_id: _,
            event: winit::event::DeviceEvent::MouseMotion { delta },
        } = event
        {
            self.egui.state.on_mouse_motion(*delta);
        }
    }

    // pub fn get_screen_descriptor(&self) -> ScreenDescriptor {
    //     ScreenDescriptor { size_in_pixels: , pixels_per_point: () }
    // }

    // Attempts to restrict the mouse movement to inside the window
    //
    // # Errors:
    // This function can fail for a number of reasons, a common one might be that the mouse is already grabbed by another application or the OS
    // this does happen occasionally such as if the user grabs the title bar of the window to drag it around on many Linux machines
    // so be a little careful on when you try to grab the mouse, such as when receiving focus.
    // pub fn set_mouse_grabbed(&self, grabbed: bool) -> Result<(), ExternalError> {
    //     let gl_win = self.dis.gl_window();
    //     let win = gl_win.window();
    //
    //     win.set_cursor_grab(grabbed)
    // }

    // Sets the mouse visible or invisible
    // pub fn set_mouse_visible(&self, visible: bool) {
    //     let gl_win = self.dis.gl_window();
    //     let win = gl_win.window();
    //
    //     win.set_cursor_visible(visible);
    // }
}

impl EguiManager {
    /// Setup everything required to render Egui
    pub fn new<T>(
        device: &wgpu::Device,
        texture_format: TextureFormat,
        event_loop: &EventLoopWindowTarget<T>,
    ) -> Self {
        Self {
            renderer: egui_wgpu::Renderer::new(device, texture_format, None, 1),
            state: egui_winit::State::new(
                egui::Context::default(),
                ViewportId::ROOT,
                &event_loop,
                None,
                Some(device.limits().max_texture_dimension_2d as usize),
            ),
        }
    }

    /// Update egui state
    pub fn on_event(
        &mut self,
        window: &winit::window::Window,
        event: &WindowEvent,
    ) -> EventResponse {
        self.state.on_window_event(window, event)
    }

    /// Render the `run_ui` to the `output` texture using Egui.
    /// Requires a view and encoder to be already instantiated.
    ///
    /// # Example
    /// ```
    /// let output = ctx.wgpu_state.surface.get_current_texture()?;
    /// let view = output
    ///     .texture
    ///     .create_view(&wgpu::TextureViewDescriptor::default());
    ///
    /// let mut encoder = ctx
    ///     .wgpu_state
    ///     .device
    ///     .create_command_encoder(&wgpu::CommandEncoderDescriptor {
    ///         label: Some("Render Encoder");
    ///     });
    ///
    /// ctx.egui.render(&mut ctx.wgpu_state, &view, &mut encoder, |gui_ctx| {
    ///     egui::Window::new("Hello").show(gui_ctx, |ui| {
    ///         ui.heading("World!");
    ///     })
    /// });
    ///
    /// // Render
    /// ctx.wgpu_state_queue.submit([encoder.finish()]);
    /// output.present();
    /// Ok(())
    /// ```
    pub fn render(
        &mut self,
        wgpu_state: &mut WgpuState,
        view: &TextureView,
        encoder: &mut CommandEncoder,
        run_ui: impl FnOnce(&egui::Context),
    ) {
        let input = self.state.take_egui_input(wgpu_state.window);
        let run_output = self.state.egui_ctx().run(input, run_ui);
        self.state
            .handle_platform_output(wgpu_state.window, run_output.platform_output);

        let screen_descriptor = ScreenDescriptor {
            size_in_pixels: [wgpu_state.config.width, wgpu_state.config.height],
            #[allow(clippy::cast_possible_truncation)]
            pixels_per_point: wgpu_state.window.scale_factor() as f32,
        };

        let clipped_primitives = self
            .state
            .egui_ctx()
            .tessellate(run_output.shapes, self.state.egui_ctx().pixels_per_point());

        for (id, image_delta) in &run_output.textures_delta.set {
            self.renderer
                .update_texture(&wgpu_state.device, &wgpu_state.queue, *id, image_delta);
        }

        let command_buffer = self.renderer.update_buffers(
            &wgpu_state.device,
            &wgpu_state.queue,
            encoder,
            &clipped_primitives,
            &screen_descriptor,
        );

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Egui render pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                // depth_stencil_attachment: Some(depth_attachment),
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            self.renderer
                .render(&mut render_pass, &clipped_primitives, &screen_descriptor);
        }

        for id in &run_output.textures_delta.free {
            self.renderer.free_texture(id);
        }

        wgpu_state.queue.submit(command_buffer);
    }
}
