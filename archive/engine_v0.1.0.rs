//Ryan Berg 6/14/22
// mod window_events;

use wgpu::util::DeviceExt;
use winit::{
    event::{Event , WindowEvent},
    event_loop::{ControlFlow, EventLoop},
};
use nanorand::{Rng, WyRand};
use std::{borrow::Cow, mem};

const MAX_AGENT_COUNT:u32 = 10000;
const AGENTS_PER_WORK_GROUP: u32 = 64;

pub async fn init(zone_width: u32)
{
    let event_loop = EventLoop::new();
    let window = winit::window::WindowBuilder::new()
        .with_title("Zone 3")
        .with_inner_size(winit::dpi::PhysicalSize::new(zone_width, zone_width))
        .build(&event_loop)
        .unwrap();

    let web_gpu = wgpu::Instance::new(wgpu::Backends::all());
    let surface = unsafe {web_gpu.create_surface(&window)};

    let adapter = web_gpu.request_adapter(&wgpu::RequestAdapterOptions{
        power_preference: wgpu::PowerPreference::HighPerformance,
        compatible_surface: Some(&surface),
        force_fallback_adapter: false
    }).await.expect("Could not find adapter.");


    let (device, queue) = adapter.request_device(&wgpu::DeviceDescriptor{
        label: None,
        features: wgpu::Features::empty(),
        limits: wgpu::Limits::default()
    }, None).await.expect("Could not create device");



    let draw_shader = device.create_shader_module(&wgpu::ShaderModuleDescriptor {
        label: None,
        source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("draw.wgsl"))),
    });

    let compute_shader = device.create_shader_module(&wgpu::ShaderModuleDescriptor {
        label: None,
        source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("flock.wgsl"))),
    });

    let sim_parameters = [
        0.04f32, // deltaT
        0.1,     // rule1Distance
        0.025,   // rule2Distance
        0.025,   // rule3Distance
        0.02,    // rule1Scale
        0.05,    // rule2Scale
        0.005,   // rule3Scale
    ].to_vec();

    let sim_param_buffer = device.create_buffer_init( &wgpu::util::BufferInitDescriptor{
        label: Some("Sim Parameter Buffer"),
        contents: bytemuck::cast_slice(&sim_parameters),
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST
    });

    let compute_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor{
        label: None,
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0, //sim parameters
                count: None,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: wgpu::BufferSize::new(
                        (sim_parameters.len() * mem::size_of::<f32>()) as _,
                    ),
                }
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1, //input agent attributes
                count: None,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: wgpu::BufferSize::new((MAX_AGENT_COUNT * 16) as _) //ToDo: why 16?
                }
            },
            wgpu::BindGroupLayoutEntry {
                binding: 2, //output agent attributes
                count: None,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: false },
                    has_dynamic_offset: false,
                    min_binding_size: wgpu::BufferSize::new((MAX_AGENT_COUNT * 16) as _)
                }
            }
        ]
    });
    
    let compute_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor{
       label: Some("Compute"),
       bind_group_layouts: &[&compute_bind_group_layout],
       push_constant_ranges: &[]
    });
    
    let compute_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor{
        label: Some("Compute Pipeline"),
        layout: Some(&compute_pipeline_layout),
        module: &compute_shader,
        entry_point: "main"
    });
    
    

    let render_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Render"),
            bind_group_layouts: &[],
            push_constant_ranges: &[],
        });
    
    let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor{
        label: None,
        layout: Some(&render_pipeline_layout),
        vertex: wgpu::VertexState {
            module: &draw_shader,
            entry_point: "vs_main",
            buffers: &[
                wgpu::VertexBufferLayout {
                    array_stride: 4 * 4,
                    step_mode: wgpu::VertexStepMode::Instance,
                    attributes: &wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x2],
                },
                wgpu::VertexBufferLayout {
                    array_stride: 2 * 4,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![2 => Float32x2],
                }
            ]
        },
        fragment: Some(wgpu::FragmentState {
            module: &draw_shader,
            entry_point: "fs_main",
            targets: &[surface.get_preferred_format(&adapter).unwrap().into()],
        }),
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        multiview: None,
    });

    //object shape
    let vertex_buffer_data = [-0.01f32, -0.02, 0.01, -0.02, 0.00, 0.02];
    let vertices_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Vertex Buffer"),
        contents: bytemuck::bytes_of(&vertex_buffer_data),
        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
    });

    //object associated data
    let mut initial_agent_data = vec![0.0f32; (4 * MAX_AGENT_COUNT) as usize];
    let mut rng = WyRand::new_seed(42);
    let mut unif = || rng.generate::<f32>() * 2f32 - 1f32; // Generate a num (-1, 1)
    for agent_instance_chunk in initial_agent_data.chunks_mut(4) {
        agent_instance_chunk[0] = unif(); // posx
        agent_instance_chunk[1] = unif(); // posy
        agent_instance_chunk[2] = unif() * 0.1; // velx
        agent_instance_chunk[3] = unif() * 0.1; // vely
    }

    // creates two buffers of agent data each of size MAX_AGENT_COUNT
    // the two buffers alternate as dst and src for each frame

    let mut agent_buffers = Vec::<wgpu::Buffer>::new();
    let mut agent_bind_groups = Vec::<wgpu::BindGroup>::new();
    for i in 0..2 {
        agent_buffers.push(
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(&format!("Agent Buffer {}", i)),
                contents: bytemuck::cast_slice(&initial_agent_data),
                usage: wgpu::BufferUsages::VERTEX
                    | wgpu::BufferUsages::STORAGE
                    | wgpu::BufferUsages::COPY_DST,
            }),
        );
    }

    // create two bind groups, one for each buffer as the src
    // where the alternate buffer is used as the dst
    //ToDo: what is a bind group for?
    for i in 0..2
    {
        agent_bind_groups.push(device.create_bind_group(&wgpu::BindGroupDescriptor{
            label: None,
            layout: &compute_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry{
                    binding: 0,
                    resource: sim_param_buffer.as_entire_binding()
                },
                wgpu::BindGroupEntry{
                    binding: 1,
                    resource: agent_buffers[i].as_entire_binding()
                },
                wgpu::BindGroupEntry{
                    binding: 2,
                    resource: agent_buffers[(i+1)%2].as_entire_binding()
                }
            ]
        }))
    }



    let mut surface_config = wgpu::SurfaceConfiguration {
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        format: surface.get_preferred_format(&adapter).unwrap(),
        width: window.inner_size().width,
        height: window.inner_size().height,
        present_mode: wgpu::PresentMode::Mailbox
    };
    surface.configure(&device, &surface_config);


    // calculates number of work groups from PARTICLES_PER_GROUP constant
    let work_group_count = ((MAX_AGENT_COUNT as f32) / (AGENTS_PER_WORK_GROUP as f32)).ceil() as u32;

    let mut frame_count:usize = 0;
    event_loop.run(move |event, _, control_flow| {
        // Have the closure take ownership of the resources.
        // `event_loop.run` never returns, therefore we must do this to ensure
        // the resources are properly cleaned up.
        let _ = (&web_gpu, &adapter, &draw_shader, &render_pipeline_layout);

        *control_flow = ControlFlow::Wait;
        match event {
            Event::WindowEvent {
                event: WindowEvent::Resized(size),
                ..
            } => {
                // Reconfigure the surface with the new size
                surface_config.width = size.width;
                surface_config.height = size.height;
                surface.configure(&device, &surface_config);
                // On macos the window needs to be redrawn manually after resizing
                window.request_redraw();
            }
            Event::RedrawRequested(_) => {
                let frame = surface
                    .get_current_texture()
                    .expect("Failed to acquire next swap chain texture");
                let view = frame
                    .texture
                    .create_view(&wgpu::TextureViewDescriptor::default());

                let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

                {
                    let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor{label:Some("Compute Pass")});

                    compute_pass.set_pipeline(&compute_pipeline);
                    compute_pass.set_bind_group(0, &agent_bind_groups[frame_count%2], &[]);
                    compute_pass.dispatch_workgroups(work_group_count, 1, 1);
                }

                {
                    let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: None,
                        color_attachments: &[wgpu::RenderPassColorAttachment {
                            view: &view,
                            resolve_target: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Clear(wgpu::Color{r:0.2588, g: 0., b: 0., a: 1.}),
                                store: true,
                            },
                        }],
                        depth_stencil_attachment: None,
                    });
                    {
                        rpass.set_pipeline(&render_pipeline);
                        // render dst agents
                        rpass.set_vertex_buffer(0, agent_buffers[((frame_count + 1) % 2)].slice(..));
                        // the three instance-local vertices
                        rpass.set_vertex_buffer(1, vertices_buffer.slice(..));
                        rpass.draw(0..3, 0..MAX_AGENT_COUNT);
                    }

                }

                frame_count += 1;
                queue.submit(Some(encoder.finish()));
                frame.present();
                window.request_redraw();
            }
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => *control_flow = ControlFlow::Exit,
            _ => {}
        }
    });
}















