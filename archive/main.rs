#![feature(core)]
#![feature(portable_simd)]
use core_simd::*;//requires nightly

mod engine_v0_1_2;

fn main()
{
    #[cfg(not(target_arch = "wasm32"))]
    {
        env_logger::init();
        pollster::block_on(engine_v0_1_2::init(3200, 1600));
    }
    #[cfg(target_arch = "wasm32")]
    {
        std::panic::set_hook(Box::new(console_error_panic_hook::hook));
        console_log::init().expect("could not initialize logger");
        wasm_bindgen_futures::spawn_local(engine::init(300));
    }
}