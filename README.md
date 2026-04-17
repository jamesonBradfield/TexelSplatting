# TexelSplatting

Implementing individualkex's rendering strategy in Godot-Rust (`gdext`).

## The Goal

The core objective is to implement a dual-probe rendering strategy to achieve a unique, stylized visual effect. The architecture separates the actual level geometry from what the player sees, using a low-resolution cubemap as the Single Source of Truth (SSOT) for environment data.

### The Dual-Probe Strategy

1. **Probe 1 (The Environment Probe)**: 
   - Follows the player's perspective.
   - Captures the actual level geometry (e.g., rendered on Layer 1) and parses it into a lower-resolution cubemap.
   - Acts as the SSOT for all environment lighting and color data.

2. **Probe 2 (The Projection Probe)**: 
   - Generates a physical quad for *every single texel* present in Probe 1's cubemap.
   - Projects these quads onto a depth buffer grabbed from the player's perspective.
   - Renders these projected quads into a separate, culled "Fake World" (e.g., Layer 2).

### Visibility & Layers
The player's actual camera **only sees the Fake World (Layer 2)**. The real level geometry (Layer 1) is completely hidden from the player and exists solely to be sampled by Probe 1. This creates a distinct, texel-snapped, reprojected aesthetic where the world is reconstructed entirely from the cubemap's texels.

## Current State

*   **Environment**: Godot 4 + Rust (`gdext`) integration is active.
*   **Legacy Architecture**: The original architectural vision (the "Ghost-Grid", "Trinity of Threads", and "Zest Protocols") has been archived. See `STARTING_VIBE.md` for the historical context, initial prototype notes, and technical debt warnings.

## Development Notes

*   **Build System**: Always use `./build.sh` to compile the Rust extension and copy artifacts, avoiding Windows file lock issues with the Godot editor.
*   **Memory Management**: Strict adherence to freeing RIDs in Rust's `Drop` trait is required to prevent memory leaks when interacting with the `RenderingServer`.
