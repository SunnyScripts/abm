//Created by Ryan Berg 7/7/22
struct Agent {
  state : u32,
  zone : u32,
  x_coord : u32,
  y_coord : u32,
  life : u32,
};

struct AgentGridBin {
  occupant_bit_flags : u32,
  tcell_wander_count : u32,
  tcell_chase_cytokine_count : u32,
  dendritic_promote_inflamation_count : u32,
  dendritic_down_regulate_inflamation_count : u32,
};

struct SignalGridBin {
  occupant_bit_flags : u32,
  cytokine_signal_strength : u32,
  antibody_signal_strength : u32,
};

@group(0) @binding(0) var<storage, read> agent_count_src : u32;//in agent list
@group(0) @binding(1) var<storage, read_write> agent_count_dst : u32;

@group(1) @binding(0) var<storage, read> agent_list_src : array<Agent>;
@group(1) @binding(1) var<storage, read_write> agent_list_dst : array<Agent>;

@group(2) @binding(0) var<storage, read> zone1_agent_grid_occupants_src : array<AgentGridBin>;
@group(2) @binding(1) var<storage, read_write> zone1_agent_grid_occupants_dst : array<AgentGridBin>;
@group(2) @binding(2) var<storage, read> zone2_agent_grid_occupants_src : array<AgentGridBin>;
@group(2) @binding(3) var<storage, read_write> zone2_agent_grid_occupants_dst : array<AgentGridBin>;
@group(2) @binding(4) var<storage, read> zone3_agent_grid_occupants_src : array<AgentGridBin>;
@group(2) @binding(5) var<storage, read_write> zone3_agent_grid_occupants_dst : array<AgentGridBin>;

@group(3) @binding(0) var<storage, read> zone1_signal_grid_occupants_src : array<SignalGridBin>;
@group(3) @binding(2) var<storage, read> zone2_signal_grid_occupants_src : array<SignalGridBin>;
@group(3) @binding(4) var<storage, read> zone3_signal_grid_occupants_src : array<SignalGridBin>;

@compute
@workgroup_size(64)
fn main(@builtin(global_invocation_id) global_invocation_id: vec3<u32>) {

  let total = arrayLength(&agent_list_src);
  let index = global_invocation_id.x;
  if (index >= total) {
    return;
  }

  let plus_one = agent_list_src[0].state + u32(1);

  agent_list_dst[0].state = plus_one;

//  textureStore(storage_texture, vec2<i32>(global_invocation_id.xy), vec4<f32>(0., 1., .5, d));
}
