//Created by Ryan Berg 7/5/22
//ToDo: use async functions

use bitflags::bitflags;
use wgpu::{BufferAddress, Extent3d};
use std::mem;
use imgui::TextureId;
use wgpu::util::DeviceExt;
use crate::{Shaders, UniformData};
use imgui_wgpu::Renderer;
use rand::{Rng, SeedableRng};
use rand_chacha::{ChaCha8Rng};

bitflags! {
    struct AgentType: u32 {
        const None = 0;
        const PORTAL = 1 << 0;
        const TCELL_WANDER = 1 << 1;
        const TCELL_CHASE_CYTOKINE = 1 << 2;
        const DENDRIDIC_PROMOTE_INFLAMATION = 1 << 3;
        const DENDRIDIC_DOWN_REGULATE_INFLAMATION = 1 << 4;
    }
}

bitflags! {
    struct SignalType: u32 {
        const NONE = 0;
        const CYTOKINE = 1 << 0;
        const ANTIBODY = 1 << 1;
    }
}

pub struct GPUTasks {
    active_frame: bool,

    agent_compute_pipeline: wgpu::ComputePipeline,
    compute_diffuse_pipeline: wgpu::ComputePipeline,
    render_pipeline: wgpu::RenderPipeline,

    agent_list_bind_groups: Vec<wgpu::BindGroup>,
    agent_grid_bind_groups: Vec<wgpu::BindGroup>,
    agent_grid_buffers: Vec<wgpu::Buffer>,
    signal_grid_bind_groups: Vec<wgpu::BindGroup>,
    agent_count_bind_groups: Vec<wgpu::BindGroup>,

    pub uniform_buffer: wgpu::Buffer,
    // uniform_bind_group: wgpu::BindGroup,
    global_bind_group: wgpu::BindGroup,

    vertex_buffer: wgpu::Buffer,
    vertex_count: u32,
    instance_count: u32,

    signal_buffers: Vec<wgpu::Buffer>,
    storage_texture_bind_group: wgpu::BindGroup,
    sampler_bind_group: wgpu::BindGroup,
    read_texture_zone_1: wgpu::Texture,
    storage_texture_zone_1: wgpu::Texture,
    read_texture_zone_2: wgpu::Texture,
    storage_texture_zone_2: wgpu::Texture,
    read_texture_zone_3: wgpu::Texture,
    storage_texture_zone_3: wgpu::Texture,
    texture_size: wgpu::Extent3d,

    zone1_final_read_texture_bind_group: wgpu::BindGroup,
    zone2_final_read_texture_bind_group: wgpu::BindGroup,
    zone3_final_read_texture_bind_group: wgpu::BindGroup,
}

impl GPUTasks{
    //init
    pub fn init(uniform_buffer_data: UniformData, shader: Shaders, texture_format: wgpu::TextureFormat, device: &wgpu::Device, queue: &wgpu::Queue) -> Self{

        let vertex_buffer_data = vec![0., 0., 1., 0., 1., 1., 0., 0., 1., 0., 1., 1.];
        let empty_texture_data = vec![0u8; (4. * uniform_buffer_data.zone1_dimensions[0] * uniform_buffer_data.zone1_dimensions[1]) as usize];

        let current_agent_count = 253u32;
        let mut agent_list = vec![0f32; 5 * current_agent_count as usize];
        let mut zone2_agent_grid_occupancy_data = vec![0i32; (1. * uniform_buffer_data.zone1_dimensions[0] * uniform_buffer_data.zone1_dimensions[1]) as usize];

        let mut rng = ChaCha8Rng::seed_from_u64(2203);

        let mut agent_type = AgentType::PORTAL.bits;
        for i in 0..current_agent_count{
            if i > 9 { agent_type = AgentType::TCELL_WANDER.bits; }
            let x = (rng.gen_range(0..=100)) as f32;
            let y = (rng.gen_range(0..=100)) as f32;
            let index = ((y * 100. + x) * 1.) as usize;

            agent_list[(i * 5) as usize] = agent_type as f32;//AgentType::TCELL_WANDER.bits as f32;
            agent_list[(i * 5) as usize + 1] = (rng.gen_range(1..=3)) as f32;  //current zone
            agent_list[(i * 5) as usize + 2] = x;  //x pos
            agent_list[(i * 5) as usize + 3] = y;  //y pos
            agent_list[(i * 5) as usize + 4] = ((1+i) as f32 / (current_agent_count as f32)); //unique ID
        }

        let mut zone2_signal_grid_occupancy_data = vec![0f32; (3. * uniform_buffer_data.zone1_dimensions[0] * uniform_buffer_data.zone1_dimensions[1]) as usize];
        // let mut count = 0;

        // for grid_bin_chunk in zone2_signal_grid_occupancy_data.chunks_mut(3) {
        //     if count >= 4600 && count <= 5199 {
        //         grid_bin_chunk[0] = SignalType::CYTOKINE.bits as f32;
        //         grid_bin_chunk[1] = 1.;  //cytokine signal strength
        //         grid_bin_chunk[2] = 0.;  //antibody signal strength
        //     }
        //     count+=1;
        // }

        create_objects(6, 1,
                       shader.compute_agents, shader.compute_diffuse, shader.vert_frag_texture_sampler,
                       vertex_buffer_data, uniform_buffer_data, empty_texture_data, current_agent_count,
                       agent_list, zone2_agent_grid_occupancy_data, zone2_signal_grid_occupancy_data,
                       texture_format, device, queue)
    }

    //Compute Pass
    pub fn compute_pass(&mut self, queue: &wgpu::Queue, device: &wgpu::Device, view: &wgpu::TextureView)
    {
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("Command Encoder") });
        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor { label: Some("Agent Compute Pass") });

            compute_pass.set_pipeline(&self.agent_compute_pipeline);
            compute_pass.set_bind_group(0, &self.global_bind_group, &[]);
            compute_pass.set_bind_group(1, &self.agent_list_bind_groups[self.active_frame as u32 as usize], &[]);
            compute_pass.set_bind_group(2, &self.agent_grid_bind_groups[self.active_frame as u32 as usize], &[]);
            compute_pass.set_bind_group(3, &self.signal_grid_bind_groups[self.active_frame as u32 as usize], &[]);
            compute_pass.dispatch_workgroups(253, 1, 1);
        }
        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor { label: Some("Compute Pass") });

            compute_pass.set_pipeline(&self.compute_diffuse_pipeline);
            compute_pass.set_bind_group(0, &self.global_bind_group, &[]);
            compute_pass.set_bind_group(1, &self.agent_grid_bind_groups[self.active_frame as u32 as usize], &[]);
            compute_pass.set_bind_group(2, &self.signal_grid_bind_groups[self.active_frame as u32 as usize], &[]);
            compute_pass.set_bind_group(3, &self.storage_texture_bind_group, &[]);
            compute_pass.dispatch_workgroups(100, 100, 3);
        }
        self.toggle_active_frame();
        //ToDo: handle active frame for draw

        {
            encoder.copy_texture_to_texture(self.storage_texture_zone_1.as_image_copy(), self.read_texture_zone_1.as_image_copy(), self.texture_size);
            encoder.copy_texture_to_texture(self.storage_texture_zone_2.as_image_copy(), self.read_texture_zone_2.as_image_copy(), self.texture_size);
            encoder.copy_texture_to_texture(self.storage_texture_zone_3.as_image_copy(), self.read_texture_zone_3.as_image_copy(), self.texture_size);

        }
        queue.write_buffer(
            &self.agent_grid_buffers[((self.active_frame as u32 + 1) % 2) as usize], 0, &[0u8; 100 * 100 * 1 * 4]
        );
        queue.write_buffer(
            &self.agent_grid_buffers[((self.active_frame as u32 + 1) % 2 + 2) as usize], 0, &[0u8; 100 * 100 * 1 * 4]
        );
        queue.write_buffer(
            &self.agent_grid_buffers[((self.active_frame as u32 + 1) % 2 + 4) as usize], 0, &[0u8; 100 * 100 * 1 * 4]
        );
        queue.submit(Some(encoder.finish()));
    }

    pub fn draw(&mut self, texture_ids: [Option<TextureId>; 3], renderer: &mut Renderer, queue: &wgpu::Queue, device: &wgpu::Device){

        for (i, texture_id) in texture_ids.iter().enumerate(){
            match texture_id {
                Some(texture_id) => {
                    let active_texture_bind_group;
                    match i {
                        0 => {active_texture_bind_group = &self.zone1_final_read_texture_bind_group;}
                        1 => {active_texture_bind_group = &self.zone2_final_read_texture_bind_group;}
                        _ => {active_texture_bind_group = &self.zone3_final_read_texture_bind_group;}
                    }

                    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("Command Encoder") });
                    {
                        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                            label: None,
                            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                                view: &renderer.textures.get(*texture_id).unwrap().view(),
                                resolve_target: None,
                                ops: wgpu::Operations {
                                    load: wgpu::LoadOp::Clear(wgpu::Color{r:0., b:0., g:0., a:0.}),
                                    store: true,
                                },
                            })],
                            depth_stencil_attachment: None,
                        });

                        render_pass.set_pipeline(&self.render_pipeline);
                        render_pass.set_bind_group(0, &self.global_bind_group, &[]);
                        render_pass.set_bind_group(1, active_texture_bind_group, &[]);
                        render_pass.set_bind_group(2, &self.sampler_bind_group, &[]);
                        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
                        render_pass.draw(0..self.vertex_count, 0..self.instance_count);

                        drop(render_pass);
                    }
                    queue.submit(Some(encoder.finish()));

                }
                None => {/*fuck_all*/}
            }
        }
    }

    //Toggle Active Frame
    pub fn toggle_active_frame(&mut self) {
        if self.active_frame { self.active_frame = false }
        else { self.active_frame = true }
    }
}
//Create Objects
fn create_objects(vertex_count: u32, instance_count: u32,
                  agent_compute_shader: wgpu::ShaderModule, compute_shader: wgpu::ShaderModule, draw_shader: wgpu::ShaderModule,
                  vertex_buffer_data: Vec<f32>, uniform_buffer_data: UniformData, empty_texture_data: Vec<u8>, agent_count: u32,
                  agent_list: Vec<f32>, zone2_agent_grid_occupancy_data: Vec<i32>, zone2_signal_grid_occupancy_data: Vec<f32>,
                  texture_format: wgpu::TextureFormat,
                  device: &wgpu::Device, queue: &wgpu::Queue) -> GPUTasks
{
    let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("alt uniform buffer"),
        contents: bytemuck::cast_slice(&[uniform_buffer_data]),
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
    });

    let (_agent_count_buffers, agent_count_bind_group_layout, agent_count_bind_groups) = build_bind_group(
        device, "Agent Count", bytemuck::cast_slice(&[agent_count]), 2, wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        2, wgpu::ShaderStages::COMPUTE, true,
        wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Storage { read_only: true },
            has_dynamic_offset: false,
            min_binding_size: wgpu::BufferSize::new(
                (mem::size_of::<u32>()) as _,
            )
        });


    let (agent_list_buffers, agent_list_bind_group_layout, agent_list_bind_groups) = build_bind_group(
        device, "Agent List", bytemuck::cast_slice(&agent_list), 2, wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::COPY_SRC,
        2, wgpu::ShaderStages::COMPUTE, true,
        wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Storage { read_only: true },
            has_dynamic_offset: false,
            min_binding_size: wgpu::BufferSize::new(
                (agent_list.len() * mem::size_of::<u32>()) as _,
            )
        });

    let (agent_grid_buffers, agent_grid_bind_group_layout, agent_grid_bind_groups) = build_bind_group(
        device, "Agent Grid", bytemuck::cast_slice(&zone2_agent_grid_occupancy_data), 6,
        wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST |  wgpu::BufferUsages::COPY_SRC,
        6, wgpu::ShaderStages::COMPUTE, true,
        wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Storage { read_only: true },
            has_dynamic_offset: false,
            min_binding_size: wgpu::BufferSize::new(
                (zone2_agent_grid_occupancy_data.len() * mem::size_of::<u32>()) as _,
            )
        });

    let (signal_buffers, signal_grid_bind_group_layout, signal_grid_bind_groups) = build_bind_group(
        device, "Signal Grid", bytemuck::cast_slice(&zone2_signal_grid_occupancy_data), 6,
        wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST |  wgpu::BufferUsages::COPY_SRC,
        6, wgpu::ShaderStages::COMPUTE, true,
        wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Storage { read_only: false },
            has_dynamic_offset: false,
            min_binding_size: wgpu::BufferSize::new(
                (zone2_signal_grid_occupancy_data.len() * mem::size_of::<u32>()) as _,
            )
        });


    let noise_texture_bytes = image::load_from_memory(include_bytes!("../archive/rgba_noise_256.png")).unwrap().to_rgba8();

    let noise_texture = device.create_texture(&wgpu::TextureDescriptor{
        size: Extent3d{
            width: 256, height: 256, depth_or_array_layers: 1
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::TEXTURE_BINDING| wgpu::TextureUsages::COPY_DST,
        label: Some("Noise Texture")
    });

    queue.write_texture(
        noise_texture.as_image_copy(),
        &noise_texture_bytes,
        wgpu::ImageDataLayout{
            offset: 0,
            bytes_per_row: std::num::NonZeroU32::new(256 as u32 * mem::size_of::<u32>() as u32),
            rows_per_image: std::num::NonZeroU32::new(256 as u32)
        },
        Extent3d{ width: 256, height: 256, depth_or_array_layers: 1 }
    );

    let noise_texture_view = noise_texture.create_view(&wgpu::TextureViewDescriptor::default());

    let diffuse_size = [100u32, 100];

    //Texture Bind Groups
    let texture_size = Extent3d {
        width: diffuse_size[0],
        height: diffuse_size[1],
        depth_or_array_layers: 1
    };

    let read_texture_zone_1 = device.create_texture(&wgpu::TextureDescriptor{
        size: texture_size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        label: Some("Diffuse Texture")
    });

    let storage_texture_zone_1 = device.create_texture(&wgpu::TextureDescriptor{
        size: texture_size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::COPY_SRC | wgpu::TextureUsages::COPY_DST,
        label: Some("Diffuse Texture")
    });

    let read_texture_zone_2 = device.create_texture(&wgpu::TextureDescriptor{
        size: texture_size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        label: Some("Diffuse Texture")
    });

    let storage_texture_zone_2 = device.create_texture(&wgpu::TextureDescriptor{
        size: texture_size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::COPY_SRC | wgpu::TextureUsages::COPY_DST,
        label: Some("Diffuse Texture")
    });

    let read_texture_zone_3 = device.create_texture(&wgpu::TextureDescriptor{
        size: texture_size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        label: Some("Diffuse Texture")
    });

    let storage_texture_zone_3 = device.create_texture(&wgpu::TextureDescriptor{
        size: texture_size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::COPY_SRC | wgpu::TextureUsages::COPY_DST,
        label: Some("Diffuse Texture")
    });

    queue.write_texture(
        read_texture_zone_1.as_image_copy(),
        &empty_texture_data,
        wgpu::ImageDataLayout{
            offset: 0,
            bytes_per_row: std::num::NonZeroU32::new(uniform_buffer_data.zone1_dimensions[0] as u32 * mem::size_of::<u32>() as u32),
            rows_per_image: std::num::NonZeroU32::new(uniform_buffer_data.zone1_dimensions[1] as u32)
        },
        texture_size
    );

    queue.write_texture(
        storage_texture_zone_1.as_image_copy(),
        &empty_texture_data,
        wgpu::ImageDataLayout{
            offset: 0,
            bytes_per_row: std::num::NonZeroU32::new(diffuse_size[0] * mem::size_of::<u32>() as u32),
            rows_per_image: std::num::NonZeroU32::new(diffuse_size[1])
        },
        texture_size
    );

    queue.write_texture(
        read_texture_zone_2.as_image_copy(),
        &empty_texture_data,
        wgpu::ImageDataLayout{
            offset: 0,
            bytes_per_row: std::num::NonZeroU32::new(uniform_buffer_data.zone1_dimensions[0] as u32 * mem::size_of::<u32>() as u32),
            rows_per_image: std::num::NonZeroU32::new(uniform_buffer_data.zone1_dimensions[1] as u32)
        },
        texture_size
    );

    queue.write_texture(
        storage_texture_zone_2.as_image_copy(),
        &empty_texture_data,
        wgpu::ImageDataLayout{
            offset: 0,
            bytes_per_row: std::num::NonZeroU32::new(diffuse_size[0] * mem::size_of::<u32>() as u32),
            rows_per_image: std::num::NonZeroU32::new(diffuse_size[1])
        },
        texture_size
    );

    queue.write_texture(
        read_texture_zone_3.as_image_copy(),
        &empty_texture_data,
        wgpu::ImageDataLayout{
            offset: 0,
            bytes_per_row: std::num::NonZeroU32::new(uniform_buffer_data.zone1_dimensions[0] as u32 * mem::size_of::<u32>() as u32),
            rows_per_image: std::num::NonZeroU32::new(uniform_buffer_data.zone1_dimensions[1] as u32)
        },
        texture_size
    );

    queue.write_texture(
        storage_texture_zone_3.as_image_copy(),
        &empty_texture_data,
        wgpu::ImageDataLayout{
            offset: 0,
            bytes_per_row: std::num::NonZeroU32::new(diffuse_size[0] * mem::size_of::<u32>() as u32),
            rows_per_image: std::num::NonZeroU32::new(diffuse_size[1])
        },
        texture_size
    );

    let read_texture_zone_1_view = read_texture_zone_1.create_view(&wgpu::TextureViewDescriptor::default());
    let storage_texture_zone_1_view = storage_texture_zone_1.create_view(&wgpu::TextureViewDescriptor::default());

    let read_texture_zone_2_view = read_texture_zone_2.create_view(&wgpu::TextureViewDescriptor::default());
    let storage_texture_zone_2_view = storage_texture_zone_2.create_view(&wgpu::TextureViewDescriptor::default());

    let read_texture_zone_3_view = read_texture_zone_3.create_view(&wgpu::TextureViewDescriptor::default());
    let storage_texture_zone_3_view = storage_texture_zone_3.create_view(&wgpu::TextureViewDescriptor::default());


    let diffuse_sampler = device.create_sampler(&wgpu::SamplerDescriptor{
        address_mode_u: wgpu::AddressMode::Repeat,
        address_mode_v: wgpu::AddressMode::ClampToEdge,
        address_mode_w: wgpu::AddressMode::Repeat,
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Nearest,
        mipmap_filter: wgpu::FilterMode::Nearest,
        ..Default::default()
    });

    let final_read_texture_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor{
        label: None,
        entries: &[
            wgpu::BindGroupLayoutEntry{
                binding: 0, //zone 1 read texture
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    multisampled: false,
                    view_dimension: wgpu::TextureViewDimension::D2,
                    //Bilinear Interpolation
                    sample_type: wgpu::TextureSampleType::Float {filterable: true}
                },
                count: None
            },
        ]
    });

    let storage_texture_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor{
        label: None,
        entries: &[
            wgpu::BindGroupLayoutEntry{
                binding: 0, //zone 1 write texture
                visibility:wgpu::ShaderStages::COMPUTE |  wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::StorageTexture {
                    access: wgpu::StorageTextureAccess::WriteOnly,
                    format: wgpu::TextureFormat::Rgba8Unorm,
                    view_dimension: Default::default()
                },
                count: None
            },
            wgpu::BindGroupLayoutEntry{
                binding: 1, //zone 2 write texture
                visibility:wgpu::ShaderStages::COMPUTE |  wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::StorageTexture {
                    access: wgpu::StorageTextureAccess::WriteOnly,
                    format: wgpu::TextureFormat::Rgba8Unorm,
                    view_dimension: Default::default()
                },
                count: None
            },
            wgpu::BindGroupLayoutEntry{
                binding: 2, //zone 3 write texture
                visibility:wgpu::ShaderStages::COMPUTE |  wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::StorageTexture {
                    access: wgpu::StorageTextureAccess::WriteOnly,
                    format: wgpu::TextureFormat::Rgba8Unorm,
                    view_dimension: Default::default()
                },
                count: None
            },
        ]
    });

    let sampler_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor{
        label: None,
        entries: &[
            wgpu::BindGroupLayoutEntry{
                binding: 0,  //sampler
                visibility: wgpu::ShaderStages::FRAGMENT,
                //Bilinear Interpolation
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None
            },
        ]
    });

    let zone1_final_read_texture_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor{
        label: None,
        layout: &final_read_texture_bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry{
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&read_texture_zone_1_view)
            },
        ]
    });

    let zone2_final_read_texture_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor{
        label: None,
        layout: &final_read_texture_bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry{
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&read_texture_zone_2_view)
            },
        ]
    });

    let zone3_final_read_texture_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor{
        label: None,
        layout: &final_read_texture_bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry{
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&read_texture_zone_3_view)
            },
        ]
    });

    let storage_texture_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor{
        label: None,
        layout: &storage_texture_bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry{
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&storage_texture_zone_1_view)
            },
            wgpu::BindGroupEntry{
                binding: 1,
                resource: wgpu::BindingResource::TextureView(&storage_texture_zone_2_view)
            },
            wgpu::BindGroupEntry{
                binding: 2,
                resource: wgpu::BindingResource::TextureView(&storage_texture_zone_3_view)
            },
        ]
    });

    let sampler_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor{
        label: None,
        layout: &sampler_bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry{
                binding: 0,
                resource: wgpu::BindingResource::Sampler(&diffuse_sampler)
            },
        ]
    });

    let global_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor{
        label: Some("Global Layout"),
        entries: &[
            wgpu::BindGroupLayoutEntry{
                binding: 0, //zone 1 read texture
                visibility: wgpu::ShaderStages::FRAGMENT | wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None
            },
            wgpu::BindGroupLayoutEntry{
                binding: 1, //zone 1 read texture
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Texture {
                    multisampled: false,
                    view_dimension: wgpu::TextureViewDimension::D2,
                    sample_type: wgpu::TextureSampleType::Float {filterable: false}
                },
                count: None
            },
        ]
    });

    let global_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor{
        label: None,
        layout: &global_layout,
        entries: &[
            wgpu::BindGroupEntry{
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry{
                binding: 1,
                resource: wgpu::BindingResource::TextureView(&noise_texture_view)
            },
        ]
    });

    let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: None,
        contents: bytemuck::cast_slice(&vertex_buffer_data),
        usage: wgpu::BufferUsages::VERTEX,
    });

    let agent_compute_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor{
        label: Some("Agent Compute Pipeline"),
        layout: Some(&device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor{
            label: Some("Agent Compute Pipeline Layout"),
            bind_group_layouts: &[&global_layout, &agent_list_bind_group_layout, &agent_grid_bind_group_layout, &signal_grid_bind_group_layout],
            push_constant_ranges: &[]
        })),
        module: &agent_compute_shader,
        entry_point: "main"
    });

    // Create Pipelines
    let compute_diffuse_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor{
        label: Some("Compute Diffuse Pipeline"),
        layout: Some(&device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor{
            label: Some("Compute Diffuse Pipeline Layout"),
            bind_group_layouts: &[&global_layout, &agent_grid_bind_group_layout, &signal_grid_bind_group_layout, &storage_texture_bind_group_layout],
            push_constant_ranges: &[]
        })),
        module: &compute_shader,
        entry_point: "main"
    });

    let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor{
        label: None,
        layout: Some(&device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Render Pipeline"),
            bind_group_layouts: &[&global_layout, &final_read_texture_bind_group_layout, &sampler_bind_group_layout],
            push_constant_ranges: &[],
        })),
        vertex: wgpu::VertexState {
            module: &draw_shader,
            entry_point: "vs_main",
            buffers: &[]
        },
        fragment: Some(wgpu::FragmentState {
            module: &draw_shader,
            entry_point: "fs_main",
            targets: &[Some(
                wgpu::ColorTargetState{
                    format: texture_format,
                    blend: Some(wgpu::BlendState{
                        color: wgpu::BlendComponent{
                            src_factor: wgpu::BlendFactor::SrcAlpha,
                            dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                            operation: wgpu::BlendOperation::Add,},
                        alpha: wgpu::BlendComponent::OVER
                    }),
                    write_mask: wgpu::ColorWrites::ALL,
                })
            ],
        }),
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        multiview: None,
    });


    GPUTasks{
        active_frame: false,
        compute_diffuse_pipeline,
        signal_buffers,
        sampler_bind_group,
        storage_texture_bind_group,
        zone1_final_read_texture_bind_group,
        zone2_final_read_texture_bind_group,
        zone3_final_read_texture_bind_group,

        agent_list_bind_groups,
        agent_grid_bind_groups,
        signal_grid_bind_groups,
        agent_compute_pipeline,
        agent_count_bind_groups,

        uniform_buffer,
        // uniform_bind_group: uniform_bind_group.pop().unwrap(),
        global_bind_group,

        render_pipeline,
        vertex_buffer,
        vertex_count,
        instance_count,
        read_texture_zone_1,
        storage_texture_zone_1,
        read_texture_zone_2,
        storage_texture_zone_2,
        read_texture_zone_3,
        storage_texture_zone_3,
        texture_size,
        agent_grid_buffers
    }
}
//        bind{ty:wgpu::BufferBindingType::Storage { read_only: false }};
fn build_bind_group(device: &wgpu::Device, name: &str, contents: &[u8], buffer_count: u32, usage: wgpu::BufferUsages,
                    layout_entry_count: u32, visibility:  wgpu::ShaderStages, has_read_only: bool,
                    mut layout_entry_type: wgpu::BindingType)
                    -> (Vec<wgpu::Buffer>, wgpu::BindGroupLayout, Vec<wgpu::BindGroup>)
{

    let mut buffers = Vec::<wgpu::Buffer>::new();
    for i in 0..buffer_count{
        buffers.push(
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(&format!("{} Buffer {}",name, i)),
                contents, usage,
            }),
        );
    }

    let mut layout_entries = Vec::<wgpu::BindGroupLayoutEntry>::new();
    for i in 0..layout_entry_count{
        if has_read_only && i % 2 == 1{
            //ToDo: how to change the read_only property only?
            layout_entry_type = wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Storage { read_only: false },
                has_dynamic_offset: false,
                min_binding_size: wgpu::BufferSize::new(
                    (contents.len() * mem::size_of::<u8>()) as _,
                )
            };
        }
        layout_entries.push(
            wgpu::BindGroupLayoutEntry {
                binding: i, count: None,
                visibility, ty: layout_entry_type
            }
        )
    }

    let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor{
        label: Some(&format!("{} Bind Group Layout", name)),
        entries: &*layout_entries
    });

    let mut bind_groups = Vec::<wgpu::BindGroup>::new();
    for i in 0..(has_read_only as u32 + 1)
    {
        let mut bind_group_entries = Vec::<wgpu::BindGroupEntry>::new();
        for j in 0..layout_entry_count {
            bind_group_entries.push(
                wgpu::BindGroupEntry {
                    binding: j,
                    //(double buffer) frame 0: (0, 1), (2, 3) -> frame 1: (1, 0), (3, 2)
                    resource: buffers[((i + (j % 2)) % 2 + (j - (j % 2))) as usize].as_entire_binding()
                },
            )
        }
        bind_groups.push(device.create_bind_group(&wgpu::BindGroupDescriptor{
            label: Some(&format!("{} Group {}", name, i)),
            layout: &bind_group_layout,
            entries: &*bind_group_entries
        }));
    }

    return (buffers, bind_group_layout, bind_groups)
}



























