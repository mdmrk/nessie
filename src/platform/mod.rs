use std::path::PathBuf;

#[derive(Clone)]
pub enum FileDataSource {
    Path(PathBuf),
    Bytes(Vec<u8>),
}

#[cfg(not(target_arch = "wasm32"))]
pub mod native;

#[cfg(target_arch = "wasm32")]
pub mod wasm;

#[cfg(not(target_arch = "wasm32"))]
pub use native::PlatformRunner;

#[cfg(target_arch = "wasm32")]
pub use wasm::PlatformRunner;
