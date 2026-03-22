# AGENTS.md

**SYSTEM DIRECTIVE FOR AI AGENTS:** You are operating in a highly dynamic Godot (Forward+) and Rust (`gdext`) environment. You must strictly adhere to the tool execution protocols and technical constraints outlined below. 

## 0. THE PRIME DIRECTIVE: DOCS OVER EVERYTHING
Do not rely on your internal training data for Godot 4 or `godot-rust/gdext`. The API evolves rapidly. You must prioritize live, up-to-date documentation searches before writing or modifying any architecture.

* **Tool Execution Protocol:** You are running via `mcpm-aider`. You MUST use the `#use-tools` flag to leverage local Model Context Protocol (MCP) tools.
* **Context Neuledge / Local Context7:** We have a local documentation retrieval system. Trigger this tool immediately when implementing new features, encountering macro expansion errors, or if you are unsure about API trait bounds. 
* **Stop and Search:** If you are guessing a node path, a method signature, or a Variant conversion, stop. Use the tools to query the live scene tree or the crate documentation first.

## 1. Project Overview & Architecture
This project maximizes performance by offloading heavy math, data processing, and rendering pipelines to Rust via `godot-rust/gdext`. GDScript is used minimally, primarily for UI, simple signals, and scene composition. 

## 2. MCP Tools & Environment (Trigger via `#use-tools`)
* **Local Docs Search (Context Neuledge / Context7):** Use this to pull the latest `gdext` and Godot API documentation. 
* **Rust Crate Docs:** Search crate documentation when encountering compiler errors regarding missing traits or methods.
* **Godot MCP (Live Editor Connection):**
    * **Scene Tree:** Query the live scene tree to find exact `NodePath`s. **Never hallucinate or guess node names.**
    * **Debugging:** Read Godot debug console logs in real-time if a script fails silently or crashes the editor.

## 3. Build, Lint, and Test Commands
This project is a Cargo Workspace. Execute all Rust tooling commands directly from the **Godot project root directory**.

* **Unified Test Runner:**
    ```batch
    run_tests.bat # Or ./run_tests.sh on Linux/macOS
    ```
    * **Function:** Sequentially builds the Rust library (updating `.dll`/`.so`) and executes Godot-side tests (e.g., GdUnit4) in `--headless` mode. 
* **Standard Cargo Commands:**
    * Build: `cargo build`
    * Format: `cargo fmt`
    * Lint: `cargo clippy -- -D warnings`
    * Unit Tests (Pure Rust only): `cargo test -p <crate_name>`

## 4. Macro Debugging & Transparency
Because `gdext` relies heavily on macros (`#[godot_api]`, `#[derive(GodotClass)]`), compiler errors can be cryptic.

* **Command:** `cargo expand -p <crate_name> --lib <module_name> > expanded_debug.rs`
* **Action:** If the compiler reports an error "inside a macro expansion", generate this file, read it to identify type mismatches, and **delete the file** when finished.

## 5. Memory Management & Performance (CRITICAL)
* **Manual RID Management:** When interacting directly with Godot's servers (e.g., `RenderingServer`, `PhysicsServer3D`), you are given `Rid`s. These are raw, unmanaged pointers. **You must explicitly free them** in the Rust `Drop` trait implementation to prevent memory leaks.
* **Batching:** Never loop over individual instances in a high-frequency tick. Build flat buffers (e.g., `PackedFloat32Array`) and set them all at once.
* **Thread Safety:** Godot objects (`Gd<T>`) are generally **not** `Send` or `Sync`. Do not pass them between threads. Pass plain Rust data and use `Arc<Mutex<T>>` to safely share state with the presentation thread.

## 6. GDScript & Rust Interoperability
* **Inheritance over Composition:** If a GDScript file extends a Rust class, the GDScript file **IS** that Rust class. Use `self` to call exposed Rust methods. 
* **No Shadowing:** Do not define a function in GDScript with the same name as a Rust function exposed via `#[func]`.
* **Single Source of Truth:** Avoid duplicating logic across languages.

## 7. `gdext` Trait Bounds and Borrowing Rules
* **Passing by Reference:** Most Godot API methods require passing objects by reference (e.g., `&img`).
* **Variant Conversions:** Convert to Variant and pass by reference: `mat.set_shader_parameter("name", &val.to_variant());`.
* **StringName Arguments:** The `AsArg` trait handles standard Rust string slices automatically (e.g., `get_node("Player")`). Do not manually convert.
* **Explicit Upcasting:** Use `.upcast::<TargetType>()` when calling methods on parent classes (e.g., `mat.upcast::<Material>()`).

## 8. Safety & Godot-Rust Safeguards
This project uses the `safeguards-dev-balanced` feature flag in `Cargo.toml`.
* **Behavior:** The engine will panic with a descriptive message if Rust's borrowing rules are violated at runtime.
* **Agent Action:** If a test fails with a "Safeguard Violation," analyze the lifecycle of the pointer. Do not simply `clone()` to bypass it; ensure you aren't holding a mutable borrow across yield points.
