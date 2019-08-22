use raw_window_handle::HasRawWindowHandle;
use wgpu_glyph::{GlyphBrushBuilder, Scale, Section};

fn main() -> Result<(), String> {
    env_logger::init();

    // Initialize GPU
    let instance = wgpu::Instance::new();

    let adapter = instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::HighPerformance,
    });

    let mut device = adapter.request_device(&wgpu::DeviceDescriptor {
        extensions: wgpu::Extensions {
            anisotropic_filtering: false,
        },
        limits: wgpu::Limits { max_bind_groups: 1 },
    });

    // Open window and create a surface
    let event_loop = winit::event_loop::EventLoop::new();

    let window = winit::window::WindowBuilder::new()
        .with_resizable(false)
        .build(&event_loop)
        .unwrap();

    let surface = instance.create_surface(window.raw_window_handle());

    // Prepare swap chain and depth buffer
    let render_format = wgpu::TextureFormat::Bgra8UnormSrgb;
    let mut size = window.inner_size().to_physical(window.hidpi_factor());

    let mut swap_chain = device.create_swap_chain(
        &surface,
        &wgpu::SwapChainDescriptor {
            usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT,
            format: render_format,
            width: size.width.round() as u32,
            height: size.height.round() as u32,
            present_mode: wgpu::PresentMode::Vsync,
        },
    );

    let mut depth_view = create_depth_view(&device, size);

    // Prepare glyph_brush
    let inconsolata: &[u8] = include_bytes!("Inconsolata-Regular.ttf");
    let mut glyph_brush = GlyphBrushBuilder::using_font_bytes(inconsolata)
        .depth_stencil_state(wgpu::DepthStencilStateDescriptor {
            format: wgpu::TextureFormat::Depth32Float,
            depth_write_enabled: true,
            depth_compare: wgpu::CompareFunction::Greater,
            stencil_front: wgpu::StencilStateFaceDescriptor::IGNORE,
            stencil_back: wgpu::StencilStateFaceDescriptor::IGNORE,
            stencil_read_mask: 0,
            stencil_write_mask: 0,
        })
        .build(&mut device, render_format);

    // Render loop
    event_loop.run(move |event, _, control_flow| {
        match event {
            winit::event::Event::WindowEvent {
                event: winit::event::WindowEvent::CloseRequested,
                ..
            } => *control_flow = winit::event_loop::ControlFlow::Exit,
            winit::event::Event::WindowEvent {
                event: winit::event::WindowEvent::Resized(new_size),
                ..
            } => {
                size = new_size.to_physical(window.hidpi_factor());

                swap_chain = device.create_swap_chain(
                    &surface,
                    &wgpu::SwapChainDescriptor {
                        usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT,
                        format: render_format,
                        width: size.width.round() as u32,
                        height: size.height.round() as u32,
                        present_mode: wgpu::PresentMode::Vsync,
                    },
                );

                depth_view = create_depth_view(&device, size);
            }
            winit::event::Event::EventsCleared => {
                // Get a command encoder for the current frame
                let mut encoder = device.create_command_encoder(
                    &wgpu::CommandEncoderDescriptor { todo: 0 },
                );

                // Get the next frame
                let frame = swap_chain.get_next_texture();

                // Clear frame
                {
                    let _ = encoder.begin_render_pass(
                        &wgpu::RenderPassDescriptor {
                            color_attachments: &[
                                wgpu::RenderPassColorAttachmentDescriptor {
                                    attachment: &frame.view,
                                    resolve_target: None,
                                    load_op: wgpu::LoadOp::Clear,
                                    store_op: wgpu::StoreOp::Store,
                                    clear_color: wgpu::Color {
                                        r: 0.4,
                                        g: 0.4,
                                        b: 0.4,
                                        a: 1.0,
                                    },
                                },
                            ],
                            depth_stencil_attachment: None,
                        },
                    );
                }

                // Queue text on top, it will be drawn first.
                // Depth buffer will make it appear on top.
                glyph_brush.queue(Section {
                    screen_position: (30.0, 30.0),
                    text: "On top",
                    scale: Scale::uniform(95.0),
                    color: [0.8, 0.8, 0.8, 1.0],
                    z: 0.9,
                    ..Section::default()
                });

                // Queue background text next.
                // Without a depth buffer, this text would be rendered on top of the
                // previous queued text.
                glyph_brush.queue(Section {
                    bounds: (size.width as f32, size.height as f32),
                    text: &include_str!("lipsum.txt")
                        .replace("\n\n", "")
                        .repeat(10),
                    scale: Scale::uniform(30.0),
                    color: [0.05, 0.05, 0.1, 1.0],
                    z: 0.2,
                    ..Section::default()
                });

                // Draw all the text!
                glyph_brush
                    .draw_queued(
                        &mut device,
                        &mut encoder,
                        &frame.view,
                        wgpu::RenderPassDepthStencilAttachmentDescriptor {
                            attachment: &depth_view,
                            depth_load_op: wgpu::LoadOp::Clear,
                            depth_store_op: wgpu::StoreOp::Store,
                            stencil_load_op: wgpu::LoadOp::Clear,
                            stencil_store_op: wgpu::StoreOp::Store,
                            clear_depth: -1.0,
                            clear_stencil: 0,
                        },
                        size.width.round() as u32,
                        size.height.round() as u32,
                    )
                    .expect("Draw queued");

                device.get_queue().submit(&[encoder.finish()]);
            }
            _ => {}
        }
    })
}

fn create_depth_view(
    device: &wgpu::Device,
    size: winit::dpi::PhysicalSize,
) -> wgpu::TextureView {
    let depth_texture = device.create_texture(&wgpu::TextureDescriptor {
        size: wgpu::Extent3d {
            width: size.width as u32,
            height: size.height as u32,
            depth: 1,
        },
        array_layer_count: 1,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Depth32Float,
        usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT,
    });

    depth_texture.create_default_view()
}
