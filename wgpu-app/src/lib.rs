use context::{Context, EguiManager, WgpuState};

pub mod context;
pub mod io;
pub mod timer;
pub mod utils;

use egui_winit::winit::event_loop::EventLoop;
pub use timer::Timer;

use wgpu::{Adapter, Surface};
use winit::{
    event::{self, Event},
    window::WindowBuilder,
};

/// Implement this trait to run it with `run` or `run_with_context`!
pub trait Application {
    /// This function is called after everything is setup but before the first frame is rendered
    fn init(&mut self, ctx: &mut Context);
    /// Called every frame to give the application a chance to update, the timer provides information like the time since the last frame and the current frame rate
    fn update(&mut self, t: &Timer, ctx: &mut Context);
    /// Called every frame after `Self::update` to render the applicaton
    /// # Errors
    /// Can return an error if the `wgpu::Surface` could not be written
    fn render(&mut self, t: &Timer, ctx: &mut Context) -> Result<(), wgpu::SurfaceError>;
    /// Called when the window is requested to close
    fn close(&mut self, ctx: &Context);
    /// Called a number of times between each frame with all new incoming events for the application
    fn handle_event(&mut self, ctx: &mut Context, event: &Event<()>);
}

/// Create and run a window for this application
///
/// # Arguments
///
/// * `mut app: Application` - the application you want to run with winit and Wgpu
/// * `wb: WindowBuilder` - Settings on how the window should be shaped/sized/positioned/resizable etc
///
/// # Panics
/// If no suitable surface or adapter could be found
pub fn run<A: 'static + Application>(app: A, wb: WindowBuilder) {
    let event_loop = winit::event_loop::EventLoopBuilder::new()
        .build()
        .expect("Failed to build event loop");

    let window = wb.build(&event_loop).expect("Failed to build window.");

    let mut adapter_option: Option<Adapter> = None;
    let mut surface_option: Option<Surface> = None;
    for backend in [wgpu::Backends::PRIMARY, wgpu::Backends::SECONDARY] {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            dx12_shader_compiler: wgpu::Dx12Compiler::default(),
            flags: wgpu::InstanceFlags::default(),
            gles_minor_version: wgpu::Gles3MinorVersion::default(),
        });
        let Ok(surface) = instance.create_surface(&window) else {
            log::debug!("Couldn't create surface, moving on");
            continue;
        };

        adapter_option =
            pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            }));
        surface_option = Some(surface);
        if adapter_option.is_some() {
            log::debug!("Chose backend: {:?}", backend);
            break;
        }
    }

    let adapter = adapter_option.expect("Failed to find suitable backend");
    let surface = surface_option.expect("Couldn't create a suitable surface");

    let (device, queue) = pollster::block_on(adapter.request_device(
        &wgpu::DeviceDescriptor {
            label: None,
            required_features: wgpu::Features::default(),
            required_limits: wgpu::Limits::default(),
        },
        None,
    ))
    .expect("Failed to get graphics adapter device.");

    let size = window.inner_size();
    let surface_caps = surface.get_capabilities(&adapter);

    // Shader code assumes an sRGB surface texture. Using a different
    // one will result all the colors coming out darker. If you want to support non
    // sRGB surfaces, you'll need to account for that when drawing to the frame.
    let surface_format = surface_caps
        .formats
        .iter()
        .find(|f| f.is_srgb())
        .copied()
        .unwrap_or(surface_caps.formats[0]);
    // let surface_format = TextureFormat::Rgba8UnormSrgb;
    let config = wgpu::SurfaceConfiguration {
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        format: surface_format,
        width: size.width,
        height: size.height,
        present_mode: surface_caps.present_modes[0],
        alpha_mode: surface_caps.alpha_modes[0],
        view_formats: vec![],
        desired_maximum_frame_latency: 2,
    };
    surface.configure(&device, &config);

    let wgpu_state = WgpuState {
        surface,
        device,
        queue,
        config,
        size,
        window: &window,
    };

    let egui = EguiManager::new(&wgpu_state.device, surface_format, &event_loop);

    let ctx = Context::new(wgpu_state, egui);

    run_with_context(app, ctx, event_loop);
}

/// Run a `wgpu_app` `Application` with a provided Context and `EventLoop` (usually obtained from `create`)
///
/// # Arguments
/// * `mut app: Application` - the application you want to run
/// * `mut context: Context` - A `wgpu_app` Context containing a Display, Egui object and io managers
/// * `event_loop: EventLoop<()>` - The `EventLoop` for the window
///
/// # Panics
/// On out-of-memory
pub fn run_with_context<A: 'static + Application>(
    mut app: A,
    mut context: Context,
    event_loop: EventLoop<()>,
) {
    let mut t = Timer::new();

    t.reset();
    event_loop
        .run(move |ev, control_flow| {
            match &ev {
                Event::AboutToWait => {
                    context.wgpu_state.window.request_redraw();
                }
                Event::NewEvents(cause) => {
                    if matches!(cause, event::StartCause::Init) {
                        app.init(&mut context);
                    }
                }
                Event::WindowEvent {
                    window_id: _,
                    event: event::WindowEvent::CloseRequested,
                } => {
                    app.close(&context);
                    control_flow.exit();
                }
                Event::WindowEvent {
                    window_id: _,
                    event: event::WindowEvent::RedrawRequested,
                } => {
                    // Update
                    let Some(_) = t.go() else { return };
                    app.update(&t, &mut context);
                    match app.render(&t, &mut context) {
                        Ok(()) => {}
                        Err(wgpu::SurfaceError::Lost) => {
                            context.wgpu_state.resize(context.wgpu_state.size);
                        }
                        Err(wgpu::SurfaceError::OutOfMemory) => {
                            panic!("WGPU Surface out of memory");
                        }
                        Err(e) => log::error!("{:?}", e),
                    }

                    context.mouse.next_frame();
                    context.keyboard.next_frame();
                }
                _ => {
                    context.handle_event(&ev);
                    app.handle_event(&mut context, &ev);
                }
            }
        })
        .expect("Event loop failure");
}
