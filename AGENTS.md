# AGENTS.md

This document contains essential context, commands, and code style guidelines for AI coding agents operating in the TexelSplatting repository.

## Project Overview
This project uses the Godot Engine (Forward+) integrated with Rust via the `godot-rust/gdext` library. The primary logic, specifically performance-critical rendering (like texel splatting and mass rendering), is implemented in Rust.

## Build, Lint, and Test Commands

The Rust code is located in the `.rust/` directory. Ensure you navigate to this directory (`cd .rust`) or use the `workdir` parameter for `bash` commands when executing Rust tooling.

*   **Build the GDExtension:**
    ```bash
    cd .rust && cargo build
    ```
    *This compiles the Rust code and generates the dynamic library (e.g., `texel_splatting.dll`) used by Godot.*

*   **Formatting:**
    ```bash
    cd .rust && cargo fmt
    ```

*   **Linting:**
    ```bash
    cd .rust && cargo clippy -- -D warnings
    ```

*   **Running Tests:**
    ```bash
    cd .rust && cargo test
    ```

*   **Running a Single Test:**
    ```bash
    cd .rust && cargo test <test_name>
    ```

## Code Style Guidelines & Architecture

### 1. Language & Frameworks
*   **Rust (gdext):** The core logic language. Ensure familiarity with `godot-rust` conventions.
*   **Godot 4:** The engine environment.

### 2. Imports and Annotations
*   Always include standard Godot preludes: `use godot::prelude::*;`
*   Import specific classes as needed: `use godot::classes::{Node3D, RenderingServer, ...};`
*   Use `#[derive(GodotClass)]` and `#[class(base=ClassName)]` to define Godot classes.
*   Use `#[godot_api]` on implementation blocks to expose methods to Godot.
*   Expose variables/functions using `#[var]` and `#[func]`.

### 3. Naming Conventions
*   **Structs/Classes:** `PascalCase` (e.g., `MassRenderingNode`, `TexelSplatting`).
*   **Functions/Variables/Properties:** `snake_case` (e.g., `setup_multimesh`, `visible_count`).
*   **Constants:** `SCREAMING_SNAKE_CASE`.

### 4. Memory Management & Safety (Critical)
*   **RIDs (Resource IDs):** When interacting directly with Godot's servers (e.g., `RenderingServer`), you will often use `Rid`s. These are raw pointers. **You must explicitly free them** in the `Drop` trait implementation to prevent memory leaks that will crash the editor/game.
    ```rust
    impl Drop for MyClass {
        fn drop(&mut self) {
            let mut rs = RenderingServer::singleton();
            rs.free_rid(self.my_rid);
        }
    }
    ```
*   **Thread Safety:** Godot objects (`Gd<T>`) are generally not `Send` or `Sync`. When offloading work to background threads, pass plain data. Use `Arc<Mutex<T>>` to safely share state (like transform buffers) between Rust background threads and the Godot presentation thread.

### 5. Performance & Architecture Rules ("The Iron Rule")
*   **Server-Side Rendering:** For mass object rendering (thousands of instances), bypass the standard scene tree nodes. Interact directly with the `RenderingServer` (e.g., using `multimesh_set_buffer`).
*   **Batching:** When updating transforms for a `MultiMesh`, prefer building a flat buffer (`PackedFloat32Array`) in row-major order and setting it all at once via `multimesh_set_buffer`, rather than looping over individual instances.
*   **Delta-Grip:** Never tie calculations to raw frames. Always use the `delta` time passed in `_process` or `_physics_process` to ensure consistent simulation speed regardless of framerate.
*   **Background Threads:** Do not put heavy mathematical operations in Godot's `_process` loop. Spawn Rust background threads (e.g., `std::thread` or `godot::task::spawn`) to calculate "Truth", and use the main thread purely for "Presentation" (swapping buffers).

### 6. Logging & Error Handling
*   Use `godot_print!()`, `godot_warn!()`, and `godot_error!()` for Godot-facing logging instead of `println!()`.
*   Avoid using `godot_print!` inside high-frequency loops (like the 6ms background pulse), as it will bottleneck the editor.
*   Handle `Option` and `Result` properly. Avoid unwrapping `None` or `Err` if it could crash the engine unexpectedly; use `godot_error!` and return early instead.