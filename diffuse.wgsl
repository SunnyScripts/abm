struct Particle {
  pos : vec2<f32>,
  vel : vec2<f32>,
};

struct SimParams {
  deltaT : f32,
  rule1Distance : f32,
  rule2Distance : f32,
  rule3Distance : f32,
  rule1Scale : f32,
  rule2Scale : f32,
  rule3Scale : f32,
};

@group(0) @binding(0) var<uniform> params : SimParams;
@group(0) @binding(1) var<storage, read> particlesSrc : array<Particle>;
@group(0) @binding(2) var<storage, read_write> particlesDst : array<Particle>;

// https://github.com/austinEng/Project6-Vulkan-Flocking/blob/master/data/shaders/computeparticles/particle.comp
@compute
@workgroup_size(64)
fn main(@builtin(global_invocation_id) global_invocation_id: vec3<u32>) {

    long val = (long) readMatrix.getDoubleAt(x, y);
            long sum = 0;

            sum += (long) readMatrix.getDoubleAt(prevX, prevY);
            sum += 4 * (long) readMatrix.getDoubleAt(x, prevY);
            sum += (long) readMatrix.getDoubleAt(nextX, prevY);
            sum += 4 * (long) readMatrix.getDoubleAt(prevX, y);
            sum += 4 * (long) readMatrix.getDoubleAt(nextX, y);
            sum += (long) readMatrix.getDoubleAt(prevX, nextY);
            sum += 4 * (long) readMatrix.getDoubleAt(x, nextY);
            sum += (long) readMatrix.getDoubleAt(nextX, nextY);
            sum -= 20 * val;

            double delta = sum / 20.0;

            double d = val + delta * diffCon;

            d *= evapRate;

            long newState = clamp(d, MIN, MAX);


  // Write back
   = Particle(vPos, vVel);
}
