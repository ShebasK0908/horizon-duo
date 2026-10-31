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

#[cfg(target_arch = "wasm32")]
mod browser_input;
#[cfg(any(target_arch = "wasm32", test))]
mod input;

#[cfg(target_arch = "wasm32")]
mod application;
