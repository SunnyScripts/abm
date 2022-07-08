//Created by Ryan Berg 7/5/22

use std::mem;
use wgpu::util::DeviceExt;
use wgpu::{Extent3d};

//ToDo: optimal work group size? dynamic?

pub struct Layer{
    pub active_frame: bool,
    pub zone1_final_read_texture_bind_group: wgpu::BindGroup,
    pub zone2_final_read_texture_bind_group: wgpu::BindGroup,
    pub zone3_final_read_texture_bind_group: wgpu::BindGroup,
    pub storage_texture_bind_group: wgpu::BindGroup,
    pub sampler_bind_group: wgpu::BindGroup,
    pub compute_diffuse_pipeline: wgpu::ComputePipeline,

    pub agent_list_bind_groups: Vec<wgpu::BindGroup>,
    pub agent_grid_bind_groups: Vec<wgpu::BindGroup>,
    pub signal_grid_bind_groups: Vec<wgpu::BindGroup>,
    pub agent_compute_pipeline: wgpu::ComputePipeline,
    pub agent_count_bind_groups: Vec<wgpu::BindGroup>,

    pub active_zone_buffer: wgpu::Buffer,
    pub active_zone_bind_group: wgpu::BindGroup,
    pub zone_size_bind_group: wgpu::BindGroup,

    pub render_pipeline: wgpu::RenderPipeline,
    pub vertex_buffer: wgpu::Buffer,
    pub vertex_count: u32,
    pub instance_count: u32,
    pub object_count: u32,

    pub read_texture: wgpu::Texture,
    pub storage_texture: wgpu::Texture,
    pub texture_size: Extent3d
}

impl Layer{
    pub fn new(object_count: u32, vertex_count: u32, instance_count: u32,
               agent_compute_shader: wgpu::ShaderModule, compute_shader: wgpu::ShaderModule, draw_shader: wgpu::ShaderModule,
               vertex_buffer_data: Vec<f32>, generic_buffer_data: Vec<u8>, zone_size_buffer_data: [u32; 12],
               agent_list: Vec<u32>, zone2_agent_grid_occupancy_data: Vec<u32>, zone2_signal_grid_occupancy_data: Vec<u32>,
               texture_format: wgpu::TextureFormat,
               device: &wgpu::Device, queue: &wgpu::Queue) -> Layer
    {

        let zone_sizes_buffer = device.create_buffer_init( &wgpu::util::BufferInitDescriptor{
            label: Some("Zone Size Buffer"),
            contents: bytemuck::cast_slice(&zone_size_buffer_data),
            usage: wgpu::BufferUsages::UNIFORM
        });

        let zone_sizes_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor{
            label: Some("Zone Size Bind Group Layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0, //signal strength
                    count: None,
                    visibility: wgpu::ShaderStages::COMPUTE | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: wgpu::BufferSize::new(
                            (zone_size_buffer_data.len() * mem::size_of::<u32>()) as _,
                        )
                    }
                },
            ]
        });

        let zone_size_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor{
            label: None,
            layout: &zone_sizes_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry{
                    binding: 0,
                    resource: zone_sizes_buffer.as_entire_binding()
                },
            ]
        });

        let active_zone_buffer = device.create_buffer_init( &wgpu::util::BufferInitDescriptor{
            label: Some("Active Zone Buffer"),
            contents: bytemuck::cast_slice(&[1u32]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST
        });

        let active_zone_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor{
            label: Some("Active Zone Buffer Bind Group Layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0, //signal strength
                    count: None,
                    visibility: wgpu::ShaderStages::COMPUTE | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: wgpu::BufferSize::new(
                            (mem::size_of::<u32>()) as _,
                        )
                    }
                },
            ]
        });

        let active_zone_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor{
            label: None,
            layout: &active_zone_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry{
                    binding: 0,
                    resource: active_zone_buffer.as_entire_binding()
                },
            ]
        });


        let agent_count_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor{
            label: Some("Agent Count Bind Group Layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0, //Agent List Count Read
                    count: None,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: wgpu::BufferSize::new(
                            (mem::size_of::<u32>()) as _,
                        )
                    }
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1, //Agent List Count Write
                    count: None,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: wgpu::BufferSize::new(
                            (mem::size_of::<u32>()) as _,
                        )
                    }
                },
            ]
        });

        let mut agent_count_buffers = Vec::<wgpu::Buffer>::new();
        for i in 0..2 {
            agent_count_buffers.push(
                device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some(&format!("Agent Count Buffer {}", i)),
                    contents: bytemuck::cast_slice(&[11u32]),
                    usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                }),
            );
        }

        let mut agent_count_bind_groups = Vec::<wgpu::BindGroup>::new();
        for i in 0..2
        {
            agent_count_bind_groups.push(device.create_bind_group(&wgpu::BindGroupDescriptor{
                label: None,
                layout: &agent_count_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry{
                        binding: 0,
                        resource: agent_count_buffers[i].as_entire_binding()
                    },
                    wgpu::BindGroupEntry{
                        binding: 1,
                        resource: agent_count_buffers[(i+1)%2].as_entire_binding()
                    },
                ]
            }))
        }

        let mut agent_list_buffer = Vec::<wgpu::Buffer>::new();
        for _i in 0..2 {
            agent_list_buffer.push(
                device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Agent List Buffer"),
                    contents: bytemuck::cast_slice(&agent_list),
                    usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                }),
            );
        }

        let mut agent_grid_buffers = Vec::<wgpu::Buffer>::new();
        for _i in 0..6 {
            agent_grid_buffers.push(
                device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Agent Grid Occupancy Buffer"),
                    contents: bytemuck::cast_slice(&zone2_agent_grid_occupancy_data),
                    usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                }),
            );
        }

        let mut zone2_signal_buffers = Vec::<wgpu::Buffer>::new();
        for _i in 0..6 {
            zone2_signal_buffers.push(
                device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Agent Grid Occupancy Buffer"),
                    contents: bytemuck::cast_slice(&zone2_signal_grid_occupancy_data),
                    usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                }),
            );
        }

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(&vertex_buffer_data),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let agent_list_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor{
            label: Some("Agent List Bind Group Layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0, //Agent List Read
                    count: None,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: wgpu::BufferSize::new(
                            (agent_list.len() * mem::size_of::<u32>()) as _,
                        )
                    }
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1, //Agent List Write
                    count: None,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: wgpu::BufferSize::new(
                            (agent_list.len() * mem::size_of::<u32>()) as _,
                        )
                    }
                },
            ]
        });

        //Agent Bind Groups
        let agent_grid_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor{
            label: Some("Agent Grid Bind Group Layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0, //zone 1 agent grid occupancy read
                    count: None,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: wgpu::BufferSize::new(
                            (zone2_agent_grid_occupancy_data.len() * mem::size_of::<u32>()) as _,
                        )
                    }
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1, //zone 1 agent grid occupancy write
                    count: None,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: wgpu::BufferSize::new(
                            (zone2_agent_grid_occupancy_data.len() * mem::size_of::<u32>()) as _,
                        )
                    }
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2, //zone 2 agent grid occupancy read
                    count: None,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: wgpu::BufferSize::new(
                            (zone2_agent_grid_occupancy_data.len() * mem::size_of::<u32>()) as _,
                        )
                    }
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 3, //zone 2 agent grid occupancy write
                    count: None,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: wgpu::BufferSize::new(
                            (zone2_agent_grid_occupancy_data.len() * mem::size_of::<u32>()) as _,
                        )
                    }
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 4, //zone 3 agent grid occupancy read
                    count: None,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: wgpu::BufferSize::new(
                            (zone2_agent_grid_occupancy_data.len() * mem::size_of::<u32>()) as _,
                        )
                    }
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 5, //zone 3 agent grid occupancy write
                    count: None,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: wgpu::BufferSize::new(
                            (zone2_agent_grid_occupancy_data.len() * mem::size_of::<u32>()) as _,
                        )
                    }
                },
            ]
        });

        let mut agent_list_bind_groups = Vec::<wgpu::BindGroup>::new();
        for i in 0..2
        {
            agent_list_bind_groups.push(device.create_bind_group(&wgpu::BindGroupDescriptor{
                label: Some(&format!("Agent Bind Group {}", i)),
                layout: &agent_list_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry{
                        binding: 0,
                        resource: agent_list_buffer[i].as_entire_binding()
                    },
                    wgpu::BindGroupEntry{
                        binding: 1,
                        resource: agent_list_buffer[(i+1)%2].as_entire_binding()
                    },
                ]
            }));
        }

        let mut agent_grid_bind_groups = Vec::<wgpu::BindGroup>::new();
        for i in 0..2
        {
            agent_grid_bind_groups.push(device.create_bind_group(&wgpu::BindGroupDescriptor{
                label: Some(&format!("Agent Bind Group {}", i)),
                layout: &agent_grid_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry{
                        binding: 0,
                        resource: agent_grid_buffers[i].as_entire_binding()
                    },
                    wgpu::BindGroupEntry{
                        binding: 1,
                        resource: agent_grid_buffers[(i+1)%2].as_entire_binding()
                    },
                    wgpu::BindGroupEntry{
                        binding: 2,
                        resource: agent_grid_buffers[i+2].as_entire_binding()
                    },
                    wgpu::BindGroupEntry{
                        binding: 3,
                        resource: agent_grid_buffers[(i+1)%2+2].as_entire_binding()
                    },
                    wgpu::BindGroupEntry{
                        binding: 4,
                        resource: agent_grid_buffers[i+4].as_entire_binding()
                    },
                    wgpu::BindGroupEntry{
                        binding: 5,
                        resource: agent_grid_buffers[(i+1)%2+4].as_entire_binding()
                    },
                ]
            }));
        }

        //ToDo: read and write buffers
        //Signal Bind Group Layout
        let signal_grid_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor{
            label: None,
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0, //zone 1 signal grid occupancy read
                    count: None,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: wgpu::BufferSize::new(
                            (zone2_signal_grid_occupancy_data.len() * mem::size_of::<u32>()) as _,
                        )
                    }
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1, //zone 1 signal grid occupancy write
                    count: None,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: wgpu::BufferSize::new(
                            (zone2_signal_grid_occupancy_data.len() * mem::size_of::<u32>()) as _,
                        )
                    }
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2, //zone 2 signal grid occupancy read
                    count: None,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: wgpu::BufferSize::new(
                            (zone2_signal_grid_occupancy_data.len() * mem::size_of::<u32>()) as _,
                        )
                    }
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 3, //zone 2 signal grid occupancy write
                    count: None,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: wgpu::BufferSize::new(
                            (zone2_signal_grid_occupancy_data.len() * mem::size_of::<u32>()) as _,
                        )
                    }
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 4, //zone 3 signal grid occupancy read
                    count: None,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: wgpu::BufferSize::new(
                            (zone2_signal_grid_occupancy_data.len() * mem::size_of::<u32>()) as _,
                        )
                    }
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 5, //zone 3 signal grid occupancy write
                    count: None,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: wgpu::BufferSize::new(
                            (zone2_signal_grid_occupancy_data.len() * mem::size_of::<u32>()) as _,
                        )
                    }
                },
            ]
        });

        let mut signal_grid_bind_groups = Vec::<wgpu::BindGroup>::new();
        for i in 0..2
        {
            signal_grid_bind_groups.push(device.create_bind_group(&wgpu::BindGroupDescriptor{
                label: Some(&format!("Agent Bind Group {}", i)),
                layout: &signal_grid_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry{
                        binding: 0,
                        resource: zone2_signal_buffers[i].as_entire_binding()
                    },
                    wgpu::BindGroupEntry{
                        binding: 1,
                        resource: zone2_signal_buffers[(i+1)%2].as_entire_binding()
                    },
                    wgpu::BindGroupEntry{
                        binding: 2,
                        resource: zone2_signal_buffers[i+2].as_entire_binding()
                    },
                    wgpu::BindGroupEntry{
                        binding: 3,
                        resource: zone2_signal_buffers[(i+1)%2+2].as_entire_binding()
                    },
                    wgpu::BindGroupEntry{
                        binding: 4,
                        resource: zone2_signal_buffers[i+4].as_entire_binding()
                    },
                    wgpu::BindGroupEntry{
                        binding: 5,
                        resource: zone2_signal_buffers[(i+1)%2+4].as_entire_binding()
                    },
                ]
            }));
        }

        let diffuse_bytes = include_bytes!("../test_image.png");
        let diffuse_image = image::load_from_memory(diffuse_bytes).unwrap();
        let diffuse_rgba = diffuse_image.to_rgba8();

        let diffuse_size = [100u32, 100];

        //Texture Bind Groups
        let texture_size = Extent3d {
            width: diffuse_size[0],
            height: diffuse_size[1],
            depth_or_array_layers: 1
        };

        let diffuse_texture = device.create_texture(&wgpu::TextureDescriptor{
            size: texture_size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            label: Some("Diffuse Texture")
        });

        let diffuse_storage_texture = device.create_texture(&wgpu::TextureDescriptor{
            size: texture_size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::COPY_SRC | wgpu::TextureUsages::COPY_DST,
            label: Some("Diffuse Texture")
        });

        queue.write_texture(
            diffuse_texture.as_image_copy(),
            &generic_buffer_data,
            wgpu::ImageDataLayout{
                offset: 0,
                bytes_per_row: std::num::NonZeroU32::new(diffuse_size[0] * mem::size_of::<u32>() as u32),
                rows_per_image: std::num::NonZeroU32::new(diffuse_size[1])
            },
            texture_size
        );

        queue.write_texture(
            diffuse_storage_texture.as_image_copy(),
            &generic_buffer_data,
            wgpu::ImageDataLayout{
                offset: 0,
                bytes_per_row: std::num::NonZeroU32::new(diffuse_size[0] * mem::size_of::<u32>() as u32),
                rows_per_image: std::num::NonZeroU32::new(diffuse_size[1])
            },
            texture_size
        );

        let diffuse_texture_view = diffuse_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let diffuse_storage_texture_view = diffuse_storage_texture.create_view(&wgpu::TextureViewDescriptor::default());


        let diffuse_sampler = device.create_sampler(&wgpu::SamplerDescriptor{
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
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
                    resource: wgpu::BindingResource::TextureView(&diffuse_texture_view)
                },
            ]
        });

        let zone2_final_read_texture_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor{
            label: None,
            layout: &final_read_texture_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry{
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&diffuse_texture_view)
                },
            ]
        });

        let zone3_final_read_texture_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor{
            label: None,
            layout: &final_read_texture_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry{
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&diffuse_texture_view)
                },
            ]
        });

        let storage_texture_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor{
            label: None,
            layout: &storage_texture_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry{
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&diffuse_storage_texture_view)
                },
                wgpu::BindGroupEntry{
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&diffuse_storage_texture_view)
                },
                wgpu::BindGroupEntry{
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&diffuse_storage_texture_view)
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

        let agent_compute_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor{
            label: Some("Agent Compute Pipeline"),
            layout: Some(&device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor{
                label: Some("Agent Compute Pipeline Layout"),
                bind_group_layouts: &[&agent_count_bind_group_layout, &agent_list_bind_group_layout, &agent_grid_bind_group_layout, &signal_grid_bind_group_layout],
                push_constant_ranges: &[]
            })),
            module: &agent_compute_shader,
            entry_point: "main"
        });

        //Create Pipelines
        let compute_diffuse_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor{
            label: None,
            layout: Some(&device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor{
                label: Some("Compute Diffuse"),
                bind_group_layouts: &[&zone_sizes_bind_group_layout, &agent_grid_bind_group_layout, &signal_grid_bind_group_layout, &storage_texture_bind_group_layout],
                push_constant_ranges: &[]
            })),
            module: &compute_shader,
            entry_point: "main"
        });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor{
            label: None,
            layout: Some(&device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline"),
                bind_group_layouts: &[&zone_sizes_bind_group_layout, &active_zone_bind_group_layout, &final_read_texture_bind_group_layout, &sampler_bind_group_layout],
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

        Layer{
            active_frame: false,
            compute_diffuse_pipeline,
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

            active_zone_buffer,
            active_zone_bind_group,
            zone_size_bind_group,

            render_pipeline,
            vertex_buffer,
            vertex_count,
            instance_count,
            object_count,
            read_texture: diffuse_texture,
            storage_texture: diffuse_storage_texture,
            texture_size
        }
    }
    pub fn toggle_active_frame(&mut self) {
        if self.active_frame { self.active_frame = false }
        else { self.active_frame = true }
    }

}