
struct wrapped_f32 { //to meet uniform array stride requirement
  @size(16) elem: vec2<f32>
}

struct  UniformData{
    accumulator_delta_time: vec2<f32>,
    zone1_dim: vec2<f32>,
    window1_dim: vec2<f32>,
    zone2_dim: vec2<f32>,
    window2_dim: vec2<f32>,
    zone3_dim: vec2<f32>,
//    window3_dim: vec2<f32>,
    neighbors: array<vec4<f32>, 2>,//array is the 7th item in struct to satisfy uniform offset requirement
}

@group(0) @binding(0) var<uniform> u: UniformData;
@group(1) @binding(0) var t_diffuse: texture_2d<f32>;
@group(2) @binding(0) var s_diffuse: sampler;


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

    var l_uv = frag_coord.xy;// - vec2<f32>(u.window1_width, u.window1_height)*0.5;
    l_uv = l_uv / u.window1_dim.x;

//    var l_uv = frag_coord.xy - vec2<f32>(u.window1_width, u.window1_height)*0.5;
//    l_uv = l_uv / u.window1_height;

//    l_uv = l_uv / vec2<f32>(u.window1_dim.x, u.window1_dim.y);
//    l_uv = l_uv / 5.;

//    return vec4<f32>(l_uv/5., 0., 1.);

    return textureSample(t_diffuse, s_diffuse, l_uv);
}