use godot::prelude::*;

mod mass_render;
mod realtime_probe;
mod projection_probe;
struct TexelSplatting;

#[gdextension]
unsafe impl ExtensionLibrary for TexelSplatting {}
