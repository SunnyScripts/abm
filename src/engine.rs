//Created by Ryan Berg 7/4/22

//ToDo: add imgui plot windows
mod gpu_window;
mod gpu_tasks;

use std::borrow::Cow;
use std::default::Default;
use imgui::{Condition, FontSource, im_str, TextureId};
use imgui_winit_support::{HiDpiMode, WinitPlatform};
use imgui_wgpu::{Renderer, RendererConfig, Texture, TextureConfig};
use pollster::block_on;
use std::time::Instant;
use winit::{
    dpi::LogicalSize,
    event::{ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::{Window},
};

use crate::gpu_tasks::GPUTasks;
use crate::gpu_window::GPUWindow;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct  UniformData{
    time: [f32; 2],              //[accumulator, frame_delta]
    zone1_dimensions: [f32; 2],  //[width, height]
    window1_dimensions: [f32; 2],//[width, height]
    zone2_dimensions: [f32; 2],
    window2_dimensions: [f32; 2],
    zone3_dimensions: [f32; 2],
    // window3_dimensions: [f32; 2],
    neighbors: [[f32; 4]; 9]     //[no move, south, south-east, ect...]//required stride of 16B
}

impl Default for UniformData {
    fn default() -> UniformData {
        UniformData {
            time: [0., 0.],
            zone1_dimensions: [100., 100.],
            window1_dimensions: [500., 500.],
            zone2_dimensions: [100., 100.],
            window2_dimensions: [500., 500.],
            zone3_dimensions: [100., 100.],
            // window3_dimensions: [500., 500.],
            neighbors: [[0., 0., 0., 0.],[ 1., 1., 0., 0.],[ -1., 1., 0., 0.],[ 1., 0., 0., 0.],[ 0., -1., 0., 0.],[ -1., -1., 0., 0.], [-1., 0., 0., 0.],[ 1., -1., 0., 0.], [0., 1., 0., 0.]]
        }
    }
}
//[[0., 0.],[ 0., 1.],[ 1., 1.],[ 1., 0.],[ 0., -1.],[ -1., -1.], [-1., 0.],[ 1., -1.], [-1., 1.]]
pub struct Shaders{
    pub compute_agents: wgpu::ShaderModule,
    pub compute_diffuse: wgpu::ShaderModule,
    pub vert_frag_texture_sampler: wgpu::ShaderModule,
}

fn main(){
    let mut uniform_buffer_data = UniformData{..Default::default()};

    //region WGPU Init
    env_logger::init();
    let event_loop = EventLoop::new();
    let backend = wgpu::util::backend_bits_from_env().unwrap_or_else(wgpu::Backends::all);
    let wgpu_instance = wgpu::Instance::new(backend);

    let (window, size, surface) = {
        let window = Window::new(&event_loop).unwrap();
        window.set_inner_size(LogicalSize{width: 1200., height: 600.});
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
    //endregion

    let shader = Shaders{
        compute_agents: device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Agent Compute Shader"),
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("../shaders/compute_agents.wgsl")))
        }),
        compute_diffuse: device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Diffuse Shader"),
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("../shaders/compute_diffuse.wgsl")))
        }),
        vert_frag_texture_sampler: device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Draw Signal Shader"),
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("../shaders/vert_frag_texture_sampler.wgsl")))
        })
    };

    //region IMGUI init
    let surface_config = wgpu::SurfaceConfiguration {
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        format: surface.get_supported_formats(&adapter)[0],
        width: (size.width) as u32,
        height: (size.width) as u32,
        present_mode: wgpu::PresentMode::Fifo,
    };
    surface.configure(&device, &surface_config);

    let mut imgui_context = imgui::Context::create();
    let mut platform = WinitPlatform::init(&mut imgui_context);
    platform.attach_window(imgui_context.io_mut(), &window, HiDpiMode::Default);
    imgui_context.set_ini_filename(None);
    let mut last_frame = Instant::now();
    let mut cpu_time = Instant::now();
    // let mut last_cursor = None;

    let font_size = (14. * hidpi_factor) as f32;
    imgui_context.io_mut().font_global_scale = (1. / hidpi_factor) as f32;
    imgui_context.fonts().add_font(&[FontSource::DefaultFontData {
        config: Some(imgui::FontConfig{
            oversample_h: 1,
            pixel_snap_h: true,
            size_pixels: font_size,
            ..Default::default()
        })
    }]);
    //endregion


    let mut renderer = Renderer::new(&mut imgui_context, &device, &queue, RendererConfig {
        texture_format: surface_config.format,
        ..Default::default()});

    let mut gpu_tasks = GPUTasks::init(uniform_buffer_data, shader, surface.get_supported_formats(&adapter)[0], &device, &queue);

    let mut zone1_window = GPUWindow::new([560., 30.], uniform_buffer_data.window1_dimensions,
                                          "Zone 1 (Tissue)", &mut renderer, &device);

    // let mut zone2_window = GPUWindow::new([560., 30.], uniform_buffer_data.window2_dimensions,
    //                                       "Zone 2 (Lymph Node)", &mut renderer, &device);
    // let mut zone3_window = GPUWindow::new([1120., 30.], uniform_buffer_data.window3_dimensions,
    //                                       "Zone 3 (Circulatory)", &mut renderer, &device);

    let mut values = [0.3, 0., 0., 0., 0., 0.61];


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
                uniform_buffer_data.time[1] = (now - last_frame).as_secs_f32();
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

                let mut texture_ids: [Option<TextureId>; 3] = [None; 3];

                let elapsed = cpu_time.elapsed().as_secs_f32();

                values[1] = ((elapsed.sin() + 1.) * 0.5) * 0.33 + 0.4;
                // values[0] = ((elapsed.cos() + 1.) * 0.5) * 0.25 + 0.3;
                values[2] = (((elapsed * 0.9).cos() + 1.) * 0.5) * 0.4 + 0.3;
                // values[5] = ((elapsed.sin() + 1.) * 0.5) * 0.33 + 0.4;
                values[4] = ((elapsed.cos() + 1.) * 0.5) * 0.25 + 0.3;
                values[3] = (((elapsed * 0.9).cos() + 1.) * 0.5) * 0.4 + 0.3;


                match zone1_window.update(&ui, &mut renderer, &device) {
                    Some((texture_id, new_size)) => {
                        uniform_buffer_data.window1_dimensions = new_size;
                        texture_ids[0] = Some(texture_id);
                    },
                    None => {/*_*/},
                }


                ui.plot_lines( im_str!("Cell Count"), &values).graph_size([300., 200.])
                    .scale_max(1.0)
                    .scale_min(0.0)
                    .build();

                // ui.plot_lines( im_str!("Cell Count"), &values).graph_size([300., 300.])
                //     .scale_max(1.0)
                //     .scale_min(0.0)
                //     .build();

                // match zone2_window.update(&ui, &mut renderer, &device) {
                //     Some((texture_id, new_size)) => {
                //         uniform_buffer_data.window2_dimensions = new_size;
                //         texture_ids[1] = Some(texture_id);
                //     },
                //     None => {/*_*/},
                // }
                // match zone3_window.update(&ui, &mut renderer, &device) {
                //     Some((texture_id, new_size)) => {
                //         uniform_buffer_data.window3_dimensions = new_size;
                //         texture_ids[2] = Some(texture_id);
                //     },
                //     None => {/*_*/},
                // }

                uniform_buffer_data.time[0] = cpu_time.elapsed().as_secs_f32();
                queue.write_buffer(&gpu_tasks.uniform_buffer, 0, bytemuck::cast_slice(&[uniform_buffer_data]));

                gpu_tasks.compute_pass(&queue, &device);
                gpu_tasks.draw(texture_ids, &mut renderer, &queue, &device);



                let mut encoder: wgpu::CommandEncoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
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

            }
            //endregion
            _ => ()
        }
        platform.handle_event(imgui_context.io_mut(), &window, &event);
    });
}

// queue.write_buffer(&gpu.uniform_buffer, 0, bytemuck::cast_slice(&[uniform_buffer_data]));

// let values = [0.2, 0.5, 0.9];
// ui.plot_lines(im_str!("Lines"), &values).graph_size([300., 100.])
//     .scale_max(1.0)
//     .scale_min(0.0)
//     .build();;

// if frame_count % 2 == 0{
//gpu.compute_pass(&queue, &device);
// gpu.draw(renderer.textures.get(zone1_texture.id).unwrap().view(), &queue, &device);
// }

// window.build(&ui, || {
// ui.text_wrapped(&im_str!("Index: {}", result_str));
//
// ui.separator();
//
// ui.plot_histogram(im_str!(""), &state.pi_digits).build();
// ui.plot_lines(im_str!(""), &state.pi_digits).build();
//
// ui.separator();
//
// ui.input_text(im_str!("Sequence"), &mut query)
// .resize_buffer(true)
// .build();
//
// ui.separator();
//
// state.search_button_clicked = ui.button(im_str!("Search"), [75.0, 25.0]);
// });

// if last_cursor != Some(ui.mouse_cursor()){
//     last_cursor = Some(ui.mouse_cursor());
//     platform.prepare_render(&ui, &window);
// }