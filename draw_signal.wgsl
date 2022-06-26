struct SimParams {
  deltaT : f32,
  rule1Distance : f32,
  rule2Distance : f32,
  rule3Distance : f32,
  rule1Scale : f32,
  rule2Scale : f32,
  rule3Scale : f32,
};

struct VertexInput {
    @location(0) position: vec2<f32>,
};

struct VertexOutput {
    @builtin(position) frag_coord: vec4<f32>,
};

@group(0) @binding(0) var<storage, read> signals : array<f32>;
@group(0) @binding(1) var<uniform> params : SimParams;

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    // var out: vec2<f32>;

    var x = f32(i32((vertex_index << u32(1)) & u32(2)));
    var y = f32(i32(vertex_index & u32(2)));
    var uv = vec2<f32>(x, y);
    var out = 2.0 * uv - vec2<f32>(1.0, 1.0);

    var output: VertexOutput;
    output.frag_coord = vec4<f32>(out.x, out.y, 0.0, 1.0);
    return output;

//    return (vec4<f32>(out.x, out.y, 0.0, 1.0), signals);

}

fn mBall(uv: vec2<f32>, pos: vec2<f32>, radius: f32) -> f32
{
	return radius/dot(uv-pos,uv-pos);
}

let mint = vec3<f32>(.0,1.,.5);
@fragment
fn fs_main( in: VertexOutput) -> @location(0) vec4<f32>{


    //ToDo: dynamic resolution and maintaining of the aspect ratio
    //normalize screen coordinates
    var uv: vec2<f32> = in.frag_coord.xy / vec2<f32>(1600., 1600.);
    //set origin to bottom-left
    uv.y = 1. - uv.y;

    //build local coordinate system
    var l_uv = uv * 6.;
    l_uv = l_uv;
//    var id = floor(l_uv);
//    l_uv = fract(l_uv);
//    let quadrant = sign(l_uv - 0.5);
//    l_uv = fract(l_uv *2.);
//    if(quadrant.x < 0.) {l_uv.x = 1. - l_uv.x;}
//    if(quadrant.y < 0.) {l_uv.y = 1. - l_uv.y;}

    let color_bg = vec3<f32>(0.22,0.0,0.0);
    let color_inner = vec3<f32>(1.0,1.0,0.0);
    let color_outer = vec3<f32>(0.5,0.8,0.3);

//    let s = vec2<f32>(1600., 1600.);
//    uv = ((2.*in.frag_coord) - s)/s.y;

    var mb = 0.;
    var index = u32(0);
    for(var row: i32 = 0; row < 6; row++){
        for(var column: i32 = 0; column < 6; column++){
            if(signals[index] > 0.){
                mb = mb + mBall(l_uv, vec2<f32>(f32(column) + .5, f32(row) + .5), 0.02 * 6. * signals[index] + ((sin(params.deltaT * 5.) * 2. - 1.) *.003));
            }
            index++;
        }
    }


    let mbext = color_outer * (1.- smoothstep(mb, mb+0.01, 0.5)); // 0.5 fro control the blob thickness
//    let mbin = color_inner * (1.- smoothstep(mb, mb+0.01, 0.8)); // 0.8 for control the blob kernel size

    return vec4<f32>(mbext+color_bg, 1.);

//    return vec4<f32>(mbin+mbext, 1.);
}