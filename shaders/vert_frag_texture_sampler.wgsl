//Created by Ryan Berg 7/18/22

struct  UniformData{
    accumulator_delta_time: vec2<f32>,
    zone1_dim: vec2<f32>,
    window1_dim: vec2<f32>,
    zone2_dim: vec2<f32>,
    window2_dim: vec2<f32>,
    zone3_dim: vec2<f32>,
//    window3_dim: vec2<f32>,
    neighbors: array<vec4<f32>, 9>,
    colors: array<vec4<f32>, 4>,
}

@group(0) @binding(0) var<uniform> u: UniformData;

@group(1) @binding(0) var linear_sampler: sampler;
@group(1) @binding(1) var nearest_sampler: sampler;
@group(1) @binding(2) var agent_texture: texture_2d<f32>;
@group(1) @binding(3) var diffuse_texture: texture_2d<f32>;


@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> @builtin(position) vec4<f32> {

    let m = vertex_index % u32(3) + (vertex_index / u32(3));

    var x = f32(i32((m << u32(1)) & u32(2))) - 1.;
    var y = f32(i32(m & u32(2))) - 1.;
    var uv = vec2<f32>(x, y);

    return vec4<f32>(x, y, 0.0, 1.0);;
}

@fragment
fn fs_main( @builtin(position) frag_coord: vec4<f32>) -> @location(0) vec4<f32>{

    var l_uv = frag_coord.xy;// / vec2<f32>(u.window1_dim.x, u.window1_dim.y);
    l_uv = l_uv / u.window1_dim.x;

    let initial_color = textureSample(diffuse_texture, linear_sampler, l_uv);

    let with_bg_color = vec3(mix(vec3<f32>(0.22, 0., 0.), initial_color.rgb, initial_color.a));
    let agent_overlay = with_bg_color + textureSample(agent_texture, nearest_sampler, l_uv).rgb;

    return vec4(agent_overlay, 1.);
}