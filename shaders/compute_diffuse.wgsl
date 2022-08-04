//Types
let PORTAL = 0;
let AGENT_WANDER = 1;
let AGENT_CHASE = 2;

struct  UniformData{
    time: f32,
    zone_offset: i32,
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

@group(1) @binding(0) var read_diffuse_texture_zone_1: texture_2d<f32>;
@group(1) @binding(1) var write_diffuse_texture_zone_1: texture_storage_2d<rgba8unorm, write>;
@group(1) @binding(2) var read_diffuse_texture_zone_2: texture_2d<f32>;
@group(1) @binding(3) var write_diffuse_texture_zone_2: texture_storage_2d<rgba8unorm, write>;
@group(1) @binding(4) var read_diffuse_texture_zone_3: texture_2d<f32>;
@group(1) @binding(5) var write_diffuse_texture_zone_3: texture_storage_2d<rgba8unorm, write>;


let diffuse_constant = 1.;
let evaporation_rate = .99;
let signal_color = vec3<f32>(0.5, 0.4, .1);

fn read_signal(zone: i32, coordinates: vec2<i32>) -> f32 {
    switch (zone){
        default :{
            return textureLoad(read_diffuse_texture_zone_1, coordinates, 0).a;
        }
        case 2:{
            return textureLoad(read_diffuse_texture_zone_2, coordinates, 0).a;
        }
        case 3:{
            return textureLoad(read_diffuse_texture_zone_3, coordinates, 0).a;
        }
    }
}

@compute
@workgroup_size(64)
fn main(@builtin(global_invocation_id) global_invocation_id: vec3<u32>){
//region black out offscreen
//    if(i32(global_invocation_id.y) > 98){
//
//        switch (i32(global_invocation_id.z)) {
//            default :{
//                textureStore(storage_texture_zone_1, vec2<i32>(global_invocation_id.xy),
//                            vec4<f32>(0., 0., 0., 1.));
//            }
//            case 1:{
//                textureStore(storage_texture_zone_2, vec2<i32>(global_invocation_id.xy),
//                            vec4<f32>(0., 0., 0., 1.));
//            }
//            case 2:{
//                textureStore(storage_texture_zone_3, vec2<i32>(global_invocation_id.xy),
//                            vec4<f32>(0., 0., 0., 1.));
//            }
//        }
//        return;
//    }
//endregion


    var signal_intensity = 0.;
    var zone_dimensions = u.zone1_dim;
    if(i32(global_invocation_id.z) == 1) { zone_dimensions = u.zone2_dim; }
    else if(i32(global_invocation_id.z) == 2) { zone_dimensions = u.zone3_dim; }

//    let index = global_invocation_id.y * u32(zone_dimensions.y) + global_invocation_id.x;

    signal_intensity = read_signal(i32(global_invocation_id.z)+1, vec2<i32>(global_invocation_id.xy));

    var sum = 0.;
    for(var row_offset: i32 = -1; row_offset < 2; row_offset++) {
        for(var column_offset: i32 = -1; column_offset < 2; column_offset++) {
            if(row_offset == 0 && column_offset == 0) { continue; }
            var neighbor = vec2<i32>(global_invocation_id.xy);

            neighbor.y = neighbor.y + row_offset;
            if(neighbor.y < 0) {neighbor.y = i32(zone_dimensions.y) - 1;}
            else if(neighbor.y > i32(zone_dimensions.y) - 1) {neighbor.y = 0;}

            neighbor.x = neighbor.x + column_offset;
            if(neighbor.x < 0) {neighbor.x = i32(zone_dimensions.x) - 1;}
            else if(neighbor.x > i32(zone_dimensions.x) - 1) {neighbor.x = 0;}

//            let neighbor_index = neighbor.y * i32(zone_dimensions.y) + neighbor.x;
            let neighbor_signal = read_signal(i32(global_invocation_id.z)+1, neighbor);

            if(row_offset == 0 || column_offset == 0) { sum = sum + 4. *  neighbor_signal; }
            else { sum = sum + neighbor_signal; }
        }
    }

    sum = sum - 20. * signal_intensity;
    let delta = sum / 20.;
    signal_intensity = signal_intensity + delta * diffuse_constant;
    signal_intensity = signal_intensity * evaporation_rate;
    signal_intensity = clamp(signal_intensity, 0., 1.);

    switch (i32(global_invocation_id.z)+1){
        default :{
            textureStore(write_diffuse_texture_zone_1, vec2<i32>(global_invocation_id.xy), vec4<f32>(signal_color, signal_intensity));
        }
        case 2:{
            textureStore(write_diffuse_texture_zone_2, vec2<i32>(global_invocation_id.xy), vec4<f32>(signal_color, signal_intensity));
        }
        case 3:{
            textureStore(write_diffuse_texture_zone_3, vec2<i32>(global_invocation_id.xy), vec4<f32>(signal_color, signal_intensity));
        }
    }
}
