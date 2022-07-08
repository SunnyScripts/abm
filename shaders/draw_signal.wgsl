struct ZoneSize {
    dimensions: vec2<u32>,
    placeholder: vec2<u32>,//to meet min size requirements of uniform
};

@group(0) @binding(0) var<uniform> zone_size:  array<ZoneSize, 3>;
@group(1) @binding(0) var<uniform> active_zone: u32;

@group(2) @binding(0) var t_diffuse: texture_2d<f32>;

@group(3) @binding(0) var s_diffuse: sampler;


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

    let l_uv = frag_coord.xy / vec2<f32>(512., 512.);

//    return vec4<f32>(l_uv, 0., 1.);

    return textureSample(t_diffuse, s_diffuse, l_uv / 2.);
}