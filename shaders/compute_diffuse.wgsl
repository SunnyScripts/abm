//ToDo: use z to process each zone texture. need additional texture buffers

struct AgentGridBin {
  occupant_bit_flags: i32,
//  tcell_wander_count: i32,
//  tcell_chase_cytokine_count: i32,
//  dendritic_promote_inflamation_count: i32,
//  dendritic_down_regulate_inflamation_count: i32,
};

struct SignalGridBin {
  occupant_bit_flags: f32,
  cytokine_signal_strength: f32,
  antibody_signal_strength: f32,
};

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

@group(1) @binding(0) var<storage, read> zone1_agent_grid_occupants_src: array<AgentGridBin>;
@group(1) @binding(2) var<storage, read> zone2_agent_grid_occupants_src: array<AgentGridBin>;
@group(1) @binding(4) var<storage, read> zone3_agent_grid_occupants_src: array<AgentGridBin>;

@group(1) @binding(1) var<storage, read_write> zone1_agent_grid_occupants_dst: array<AgentGridBin>;
@group(1) @binding(3) var<storage, read_write> zone2_agent_grid_occupants_dst: array<AgentGridBin>;
@group(1) @binding(5) var<storage, read_write> zone3_agent_grid_occupants_dst: array<AgentGridBin>;

@group(2) @binding(0) var<storage, read_write> zone1_signal_grid_occupants_src: array<SignalGridBin>;
@group(2) @binding(1) var<storage, read_write> zone1_signal_grid_occupants_dst: array<SignalGridBin>;
@group(2) @binding(2) var<storage, read_write> zone2_signal_grid_occupants_src: array<SignalGridBin>;
@group(2) @binding(3) var<storage, read_write> zone2_signal_grid_occupants_dst: array<SignalGridBin>;
@group(2) @binding(4) var<storage, read_write> zone3_signal_grid_occupants_src: array<SignalGridBin>;
@group(2) @binding(5) var<storage, read_write> zone3_signal_grid_occupants_dst: array<SignalGridBin>;

@group(3) @binding(0) var storage_texture_zone_1: texture_storage_2d<rgba8unorm, write>;
@group(3) @binding(1) var storage_texture_zone_2: texture_storage_2d<rgba8unorm, write>;
@group(3) @binding(2) var storage_texture_zone_3: texture_storage_2d<rgba8unorm, write>;

let diffuse_constant = 1.;
let evaporation_rate = .99;

let portal_color = vec4<f32>(0.6, .33, 0., 1.);
let agent_wander_color = vec4<f32>(.3882, .7725, .8588, 1.);
let agent_chase_color = vec4<f32>(0., 1., 0., 1.);

fn read_signal_grid(zone_index: u32, grid_index: u32) -> f32
{
    switch (i32(zone_index)){
        default :{
            return zone1_signal_grid_occupants_src[grid_index].cytokine_signal_strength;
        }
        case 1:{
            return zone2_signal_grid_occupants_src[grid_index].cytokine_signal_strength;
        }
        case 2:{
            return zone3_signal_grid_occupants_src[grid_index].cytokine_signal_strength;
        }
    }
}
fn write_signal_grid_intensity(global_invocation_id: vec3<u32>, grid_index: u32, signal_intensity: f32)
{
    switch (i32(global_invocation_id.z)){
        default :{
            zone1_signal_grid_occupants_dst[grid_index].cytokine_signal_strength = signal_intensity;
            switch (zone1_agent_grid_occupants_dst[grid_index].occupant_bit_flags){
                    case 1:{
                        textureStore(storage_texture_zone_1, vec2<i32>(global_invocation_id.xy), portal_color);
                    }
                    case 2:{
                        textureStore(storage_texture_zone_1, vec2<i32>(global_invocation_id.xy), agent_wander_color);
                    }
                    case 3:{
                        textureStore(storage_texture_zone_1, vec2<i32>(global_invocation_id.xy), agent_chase_color);
                    }
                    default :{
                        textureStore(storage_texture_zone_1, vec2<i32>(global_invocation_id.xy),
                            vec4<f32>(.22, 0., .85 * signal_intensity, 1.));
                    }
                }
        }
        case 1:{
            zone2_signal_grid_occupants_dst[grid_index].cytokine_signal_strength = signal_intensity;
            switch (zone2_agent_grid_occupants_dst[grid_index].occupant_bit_flags){
                case 1:{
                    textureStore(storage_texture_zone_2, vec2<i32>(global_invocation_id.xy), portal_color);
                }
                case 2:{
                    textureStore(storage_texture_zone_2, vec2<i32>(global_invocation_id.xy), agent_wander_color);
                }
                case 3:{
                    textureStore(storage_texture_zone_2, vec2<i32>(global_invocation_id.xy), agent_chase_color);
                }
                default :{
                    textureStore(storage_texture_zone_2, vec2<i32>(global_invocation_id.xy),
                        vec4<f32>(.22, 0., .85 * signal_intensity, 1.));
                }
            }
        }
        case 2:{
            zone3_signal_grid_occupants_dst[grid_index].cytokine_signal_strength = signal_intensity;
            switch (zone3_agent_grid_occupants_dst[grid_index].occupant_bit_flags){
                case 1:{
                    textureStore(storage_texture_zone_3, vec2<i32>(global_invocation_id.xy), portal_color);
                }
                case 2:{
                    textureStore(storage_texture_zone_3, vec2<i32>(global_invocation_id.xy), agent_wander_color);
                }
                case 3:{
                    textureStore(storage_texture_zone_3, vec2<i32>(global_invocation_id.xy), agent_chase_color);
                }
                default :{
                    textureStore(storage_texture_zone_3, vec2<i32>(global_invocation_id.xy),
                        vec4<f32>(.22, 0., .85 * signal_intensity, 1.));
                }
            }
        }
    }
}

@compute
@workgroup_size(64)
fn main(@builtin(global_invocation_id) global_invocation_id: vec3<u32>){

    if(i32(global_invocation_id.y) > 98){
        textureStore(storage_texture_zone_1, vec2<i32>(global_invocation_id.xy),
            vec4<f32>(0., 0., 0., 1.));
        return;
    }

    var signal_intensity = 0.;
    let index = global_invocation_id.y * u32(u.zone1_dim.x) + global_invocation_id.x;

    signal_intensity = read_signal_grid(global_invocation_id.z, index);

    var sum = 0.;
    for(var row_offset: i32 = -1; row_offset < 2; row_offset++) {
        for(var column_offset: i32 = -1; column_offset < 2; column_offset++) {
            if(row_offset == 0 && column_offset == 0) { continue; }
            var neighbor = vec2<i32>(global_invocation_id.xy);

            neighbor.y = neighbor.y + row_offset;
            if(neighbor.y < 0) {neighbor.y = i32(u.zone1_dim.x) - 1;}
            else if(neighbor.y > i32(u.zone1_dim.x) - 1) {neighbor.y = 0;}

            neighbor.x = neighbor.x + column_offset;
            if(neighbor.x < 0) {neighbor.x = i32(u.zone1_dim.x) - 1;}
            else if(neighbor.x > i32(u.zone1_dim.x) - 1) {neighbor.x = 0;}

            let neighbor_index = u32(neighbor.y * i32(u.zone1_dim.y) + neighbor.x);
            let neighbor_signal = read_signal_grid(global_invocation_id.z, neighbor_index);

            if(row_offset == 0 || column_offset == 0)
            { sum = sum + 4. *  neighbor_signal; }
            else
            { sum = sum + neighbor_signal; }
        }
    }

    sum = sum - 20. * signal_intensity;
    let delta = sum / 20.;
    signal_intensity = signal_intensity + delta * diffuse_constant;
    signal_intensity = signal_intensity * evaporation_rate;
    signal_intensity = clamp(signal_intensity, 0., 1.);

    write_signal_grid_intensity(global_invocation_id, index, signal_intensity);
}
