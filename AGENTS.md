# AGENTS.md

**SYSTEM DIRECTIVE FOR AI AGENTS:** You must strictly adhere to all guidelines, constraints, and tool triggers outlined in this document when operating within the `TexelSplatting` repository. Do not rely on outdated Godot 3 or early Godot 4 training data; defer to the rules and MCP tools defined below.

## Project Overview
This project uses the Godot Engine (Forward+) integrated with Rust via the `godot-rust/gdext` library. The primary logic, specifically performance-critical rendering (like texel splatting and mass rendering), is implemented in Rust.

## Build, Lint, and Test Commands
This project is set up as a Cargo Workspace. All Rust tooling commands MUST be executed directly from the Godot project root directory. Do not navigate into the `.rust/` directory to run these.

* **Build the GDExtension:**
    ```bash
    cargo build
    ```
    *This compiles the Rust code via the workspace and generates the dynamic library in the root `target/` folder.*

* **Formatting:**
    ```bash
    cargo fmt
    ```

* **Linting:**
    ```bash
    cargo clippy -- -D warnings
    ```

* **Running Tests:**
    ```bash
    cargo test
    ```

* **Running a Single Test:**
    ```bash
    cargo test -p texel_splatting --test <test_name>
    ```

---

## Code Style Guidelines & Architecture

### 1. Language & Frameworks
* **Rust (gdext):** The core logic language. Ensure familiarity with modern `godot-rust` conventions.
* **Godot 4:** The engine environment.

### 2. Imports and Annotations
* Always include standard Godot preludes: `use godot::prelude::*;`
* Import specific classes as needed: `use godot::classes::{Node3D, RenderingServer, ...};`
* Use `#[derive(GodotClass)]` and `#[class(base=ClassName)]` to define Godot classes.
* Use `#[godot_api]` on implementation blocks to expose methods to Godot.
* Expose variables/functions using `#[var]` and `#[func]`.

### 3. Naming Conventions
* **Structs/Classes:** `PascalCase` (e.g., `MassRenderingNode`, `TexelSplatting`).
* **Functions/Variables/Properties:** `snake_case` (e.g., `setup_multimesh`, `visible_count`).
* **Constants:** `SCREAMING_SNAKE_CASE`.

### 4. Memory Management & Safety (Critical)
* **RIDs (Resource IDs):** When interacting directly with Godot's servers (e.g., `RenderingServer`), you will often use `Rid`s. These are raw pointers. **You must explicitly free them** in the `Drop` trait implementation to prevent memory leaks that will crash the editor/game.
* **Thread Safety:** Godot objects (`Gd<T>`) are generally not `Send` or `Sync`. When offloading work to background threads, pass plain data. Use `Arc<Mutex<T>>` to safely share state between Rust background threads and the Godot presentation thread.

### 5. Performance & Architecture Rules ("The Iron Rule")
* **Server-Side Rendering:** For mass object rendering, bypass the standard scene tree nodes. Interact directly with the `RenderingServer` (e.g., using `multimesh_set_buffer`).
* **Batching:** When updating transforms for a `MultiMesh`, prefer building a flat buffer (`PackedFloat32Array`) in row-major order and setting it all at once via `multimesh_set_buffer`.
* **Delta-Grip:** Never tie calculations to raw frames. Always use the `delta` time passed in `_process` to ensure consistent simulation speed.
* **Background Threads:** Do not put heavy operations in Godot's `_process` loop. Spawn Rust background threads to calculate "Truth", and use the main thread purely for "Presentation".

### 6. Logging & Error Handling
* Use `godot_print!()`, `godot_warn!()`, and `godot_error!()` for Godot-facing logging instead of `println!()`.
* Avoid using `godot_print!` inside high-frequency loops.
* Handle `Option` and `Result` properly. Avoid unwrapping `None` or `Err` if it could crash the engine; use `godot_error!` and return early.

### 7. GDScript & Rust Interoperability (CRITICAL RULES)
* **RULE 1: INHERITANCE OVER COMPOSITION:** If a GDScript file extends a Rust class, the GDScript file **IS** that Rust class. DO NOT hallucinate instance variables (like `probe.my_method()`). DO call the exposed Rust methods directly via `self`.
* **RULE 2: NO NATIVE METHOD SHADOWING:** You CANNOT define a function in GDScript if a Rust function with the EXACT SAME NAME is exposed via `#[func]` in the parent class. Give them distinct names.
* **RULE 3: SINGLE SOURCE OF TRUTH:** Do not write the exact same setup logic in both languages. Pick one language for the logic and call it from the other.

### 8. `gdext` Trait Bounds and Borrowing Rules (CRITICAL RUST RULES)
The Godot 4 `gdext` API relies heavily on references and the `AsArg` trait. You must follow these rules to prevent compiler errors:
* **Passing by Reference:** Godot API methods like `set_shader`, `set_material_override`, `connect`, and array insertions (`Array::push`) require passing objects by reference.
    * **BAD:** `typed_faces.push(img)`
    * **GOOD:** `typed_faces.push(&img)`
* **Variant Conversions:** When a method expects a Variant, explicitly convert it and pass by reference.
    * **BAD:** `mat.set_shader_parameter("palette", pal.to_variant());`
    * **GOOD:** `mat.set_shader_parameter("palette", &pal.to_variant());`
* **StringName Arguments:** Do NOT use `.into()` or `StringName::from()` inside Godot methods like `get_node()`, `call()`, or `has_signal()`. The `AsArg` trait automatically converts standard Rust string slices.
    * **BAD:** `probe.call(StringName::from("update"), &[]);`
    * **GOOD:** `probe.call("update", &[]);`
* **Explicit Upcasting:** When calling `.upcast()` on classes with deep inheritance, explicitly declare the target type to avoid inference failures.
    * **BAD:** `self.set_material_override(&mat.upcast());`
    * **GOOD:** `self.set_material_override(&mat.upcast::<Material>());`

### 9. Model Context Protocol (MCP) Tools
You are connected to external MCP servers to verify facts, read live documentation, and interact with the engine. **Do not hallucinate APIs or guess scene tree structures.** Use your tools proactively:
* **Context7 (`context7`):** Pulls the absolute latest documentation from the web.
    * **Trigger:** If asked to implement a new `gdext` feature, or if you encounter a Godot 4 API you are not 100% confident about, you MUST query Context7 before writing code.
* **Rust Crate Docs (`rust-docs`):** Allows you to search and read local/remote crate documentation.
    * **Trigger:** If the Rust compiler throws an error regarding a missing method or trait implementation, use this tool to inspect the crate's actual API surface.
* **Live Godot Engine (`godot`):** Connects you directly to the running Godot editor.
    * **Trigger (Scene Tree):** If a script requires a `NodePath` or needs to interact with specific nodes, use this tool to query the live scene tree. **Never guess node names.**
    * **Trigger (Debugging):** If the user reports a runtime error, use this tool to read the Godot debug console or inspect the properties of the failing node in real-time.
