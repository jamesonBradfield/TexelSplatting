# SYSTEM DIRECTIVES: GODOT 4 + RUST (gdext)

**CRITICAL:** Do NOT guess APIs or node paths. The `gdext` API evolves rapidly. You MUST use `rg` (ripgrep) via shell commands to search local Rust documentation (`./target/doc/godot`) or the cargo registry BEFORE writing code. 
*Example:* `rg "fn set_shader_parameter" ./target/doc/godot` or `rg "impl.*ShaderMaterial" ~/.cargo/registry/`

## 🤖 SEARCHING DOCUMENTATION (MCP)
* **Primary Source:** Use the `context-mcp` (also known as `neuledge-context`) MCP tool with the library `generated-docs@latest`.
* **Usage:** When you need to understand a Godot-Rust API, call `get_docs(library="generated-docs@latest", topic="<your_topic>")`.
* **Consistency:** ALWAYS use `generated-docs@latest` as the library argument to ensure you are searching the most up-to-date documentation.
* **Complementary:** If the MCP results are insufficient, use `rg` as described above to search the local `./target/doc/godot` or `./godot-rust-docs/` directories.

## 🧠 SEQUENTIAL THINKING (MCP)
If you have the `sequentialthinking` tool enabled, use it to decompose complex problems.
* **Chaining:** If you reach the end of a thought and need more, set `nextThoughtNeeded: true`. 
* **Self-Correction:** Use subsequent thoughts to revise earlier assumptions.
* **Note on Aider/MCP Loops:** If your runner (Aider) does not automatically loop after a thought, try to pack more reasoning into each individual thought call or ask the user to "continue thinking" if you hit a turn limit.

## 1. BUILD & DEBUGGING
* **Workspace:** Run all tools from the Godot project root.
* **Linter/Validation:** Execute `cargo clippy` to check for errors and lints before committing.
* **Macro Errors:** If Rust compiler fails inside a macro, execute `cargo expand -p <crate> --lib <mod> > expanded_debug.rs`. Read it, fix the type mismatch, and delete the file.
* **Building:** ALWAYS execute `./build.sh` instead of `cargo build`. This script compiles the crate and copies the artifacts to the `bin/` directory to prevent Windows OS Error 5 (file locks) while the Godot editor is open. Do NOT run `cargo build` directly.

## 2. MEMORY & THREADING (STRICT)
* **RIDs:** Raw pointers (`RenderingServer`, etc.) MUST be explicitly freed in the Rust `Drop` trait to prevent leaks.
* **Batching:** NEVER loop over instances in a tick. Build flat buffers (`PackedFloat32Array`) and set them at once.
* **Thread Safety:** `Gd<T>` is NOT `Send`/`Sync`. Do not pass Godot objects between threads. Use `Arc<Mutex<T>>` for plain Rust data.
* **Safeguards:** `safeguards-dev-balanced` is active. A runtime panic means you violated borrowing rules. Do not `clone()` to bypass it; fix the mutable borrow.

## 3. INTEROP & TYPE CONVERSIONS
* **Inheritance:** GDScript extending a Rust class IS the Rust class. Use `self` to call exposed methods.
* **Passing References:** Most Godot API methods require passing by reference (e.g., `&img`).
* **Variants:** Convert and pass by reference: `&val.to_variant()`.
* **Strings:** Pass standard Rust string slices directly (handled automatically via `AsArg`).
* **Upcasting:** Use `.upcast::<TargetType>()` explicitly for parent class methods (e.g., `mat.upcast::<Material>()`).
* **Typed Signals:** When using `self.signals().my_signal().emit(...)`, you MUST obey ToGodot passing rules: Pass ByRef types (like Array, Gd<T>, StringName) by reference (e.g., &array). Pass ByValue types (like Rid, Vector3, i32) by value (e.g., my_rid). Do NOT append .to_variant() or .clone() to arguments in typed signal emissions.
