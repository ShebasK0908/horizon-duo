#![forbid(unsafe_code)]
#[cfg(target_arch = "wasm32")]
mod fixture;

#[cfg(target_arch = "wasm32")]
mod resources;
#[cfg(target_arch = "wasm32")]
mod socket;

#[cfg(target_arch = "wasm32")]
mod client;
#[cfg(target_arch = "wasm32")]
mod render_frame;
