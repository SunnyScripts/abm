//Created by Ryan Berg 7/4/22
//ToDo: add imgui plot windows
mod layer;
mod zone;
use crate::zone::{Zone, Shaders};

use std::borrow::Cow;
use std::default::Default;
use imgui::{Condition, FontSource};
use imgui_winit_support::{HiDpiMode, WinitPlatform};
use imgui_wgpu::{Renderer, RendererConfig, Texture, TextureConfig};
use pollster::block_on;
use std::time::Instant;
use wgpu::{Extent3d};
use winit::{
    dpi::LogicalSize,
    event::{ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::{Window},
};

fn main(){
    env_logger::init();
    let event_loop = EventLoop::new();
    let backend = wgpu::util::backend_bits_from_env().unwrap_or_else(wgpu::Backends::all);
    let wgpu_instance = wgpu::Instance::new(backend);

    let (window, size, surface) = {
        let window = Window::new(&event_loop).unwrap();
        window.set_inner_size(LogicalSize{width: 1280., height: 720.});
        window.set_title("BIS 22");

        let size = window.inner_size();
        let surface = unsafe {wgpu_instance.create_surface(&window)};

        (window, size, surface)
    };
    let hidpi_factor = window.scale_factor();
    let adapter = block_on(wgpu_instance.request_adapter(&wgpu::RequestAdapterOptions{
        power_preference: wgpu::PowerPreference::HighPerformance,
        compatible_surface: Some(&surface),
        force_fallback_adapter: false
    })).unwrap();

    let (device, queue) = block_on(adapter.request_device(&wgpu::DeviceDescriptor::default(), None)).unwrap();

    //Swap Chain ToDo: is this size affecting texture scale?
    let surface_config = wgpu::SurfaceConfiguration {
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        format: surface.get_supported_formats(&adapter)[0],
        width: (size.width) as u32,
        height: (size.width) as u32,
        present_mode: wgpu::PresentMode::Fifo,
    };
    surface.configure(&device, &surface_config);

    let shaders = Shaders{
        agent_compute: device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Agent Compute Shader"),
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("../shaders/agent_compute.wgsl")))
        }),
        signal_compute: device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Diffuse Shader"),
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("../shaders/diffuse.wgsl")))
        }),
        signal_draw: device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Draw Signal Shader"),
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("../shaders/draw_signal.wgsl")))
        })
    };

    //Initialize imgui context
    let mut imgui_context = imgui::Context::create();
    let mut platform = WinitPlatform::init(&mut imgui_context);
    platform.attach_window(imgui_context.io_mut(), &window, HiDpiMode::Default);
    imgui_context.set_ini_filename(None);

    let font_size = (13. * hidpi_factor) as f32;
    imgui_context.io_mut().font_global_scale = (1. / hidpi_factor) as f32;

    imgui_context.fonts().add_font(&[FontSource::DefaultFontData {
        config: Some(imgui::FontConfig{
            oversample_h: 1,
            pixel_snap_h: true,
            size_pixels: font_size,
            ..Default::default()
        })
    }]);

    let renderer_config = RendererConfig {
        texture_format: surface_config.format,
        ..Default::default()
    };

    let mut renderer = Renderer::new(&mut imgui_context, &device, &queue, renderer_config);
    let mut last_frame = Instant::now();
    let mut last_cursor = None;
    let mut circulatory_window_size: [f32; 2] = [512., 512.];

    let mut frame_count: usize = 0;

    let mut circulatory = Zone::new([100, 100], shaders, surface.get_supported_formats(&adapter)[0], &device, &queue);

    let texture_config = TextureConfig{
        size: Extent3d{
            width: circulatory_window_size[0] as u32,
            height: circulatory_window_size[1] as u32,
            ..Default::default()
        },
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        ..Default::default()
    };

    let texture = Texture::new(&device, &renderer, texture_config);
    let circulatory_texture_id = renderer.textures.insert(texture);

    event_loop.run(move | event, _, control_flow |{
        *control_flow = if cfg!(feature = "metal-auto-capture"){
            ControlFlow::Exit
        }
        else{
            ControlFlow::Poll
        };
        match event{
        //region Window Resize
            Event::WindowEvent {
                event: WindowEvent::Resized(_),
                ..
            } => {
                let size = window.inner_size();

                let surface_desc = wgpu::SurfaceConfiguration {
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                    format: surface.get_supported_formats(&adapter)[0],
                    width: size.width as u32,
                    height: size.height as u32,
                    present_mode: wgpu::PresentMode::Fifo,
                };

                surface.configure(&device, &surface_desc);
            }
            //endregion
        //region Keyboard Input
            Event::WindowEvent {
                event:
                WindowEvent::KeyboardInput {
                    input:
                    KeyboardInput {
                        virtual_keycode: Some(VirtualKeyCode::Escape),
                        state: ElementState::Pressed,
                        ..
                    },
                    ..
                },
                ..
            }
            | Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                *control_flow = ControlFlow::Exit;
            }
            //endregion
            Event::MainEventsCleared => window.request_redraw(),
        //region Redraw
            Event::RedrawEventsCleared => {
                let now = Instant::now();
                imgui_context.io_mut().update_delta_time(now - last_frame);
                last_frame = now;

                let frame = match surface.get_current_texture(){
                    Ok(frame) => frame,
                    Err(error_event) =>{
                        eprintln!("Frame Dropped: {:?}", error_event);
                        return;
                    }
                };

                platform.prepare_frame(imgui_context.io_mut(), &window).expect("Frame preparation failed.");
                let ui = imgui_context.frame();
                let view = frame.texture.create_view(&wgpu::TextureViewDescriptor::default());

                let mut updated_circulatory_window_size: Option<[f32; 2]> = None;

                //Build Window
                imgui::Window::new("Circulatory (Zone 3)")
                    .position([30., 30.], Condition::FirstUseEver)
                    .size([512., 512.], Condition::FirstUseEver)
                    .build(&ui, ||{
                        updated_circulatory_window_size = Some(ui.content_region_avail());
                        imgui::Image::new(circulatory_texture_id, updated_circulatory_window_size.unwrap()).build(&ui);
                    });

                //Resize Window
                if let Some(window_size) = updated_circulatory_window_size {
                    if window_size != circulatory_window_size && window_size[0] >= 1. && window_size[1] >= 1. {
                        circulatory_window_size = window_size;

                        let scale = &ui.io().display_framebuffer_scale;
                        renderer.textures.replace(circulatory_texture_id, Texture::new(
                            &device, &renderer, TextureConfig {
                                size: Extent3d {
                                    width: (circulatory_window_size[0] * scale[0]) as u32,
                                    height: (circulatory_window_size[1] * scale[1]) as u32,
                                    ..Default::default()
                                },
                                usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
                                ..Default::default()
                            }
                        ));
                    }

                    // if frame_count % 2 == 0{
                        circulatory.compute_pass(&queue, &device);
                        circulatory.draw(3, renderer.textures.get(circulatory_texture_id).unwrap().view(), &queue, &device);
                    // }
                }

                //Update GUI
                let mut encoder: wgpu::CommandEncoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

                if last_cursor != Some(ui.mouse_cursor()){
                    last_cursor = Some(ui.mouse_cursor());
                    platform.prepare_render(&ui, &window);
                }

                let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("GUI Render Pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color { r: 0.7, g: 0.4, b: 0.1, a: 0. }),
                            store: true,
                        },
                    })],
                    depth_stencil_attachment: None,
                });

                renderer.render(ui.render(), &queue, &device, &mut render_pass)
                    .expect("Final Render Failed.");

                drop(render_pass);
                queue.submit(Some(encoder.finish()));
                frame.present();
                // frame_count+=1;
            }
            //endregion
            _ => ()
        }
        platform.handle_event(imgui_context.io_mut(), &window, &event);
    });
}