# WebGPU / Rust based Immune Simmulation
This is an attempt to replicate the findings of Virginia Folcik’s paper on Basic Immune Simulation.
Significant performance gains were seen early on as agents interacted with each other with dynamic behavior driven by diffusion based signaling pathways, proximity and location. Using WebGPU in this way allowed for portability across all systems supporting compute shaders.
An exact 1 to 1 result could not be acheived due to differences in the handeling of random number generation without falling back to CPU based methods.
