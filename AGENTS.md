# SYSTEM DIRECTIVES: GODOT 4 + RUST (gdext)

**CRITICAL:** Do NOT guess APIs or node paths. The `gdext` API evolves rapidly. You MUST use `#use-tools` to search `godot-rust` docs or query the live scene tree via MCP BEFORE writing code.

## 1. BUILD & DEBUGGING
* **Workspace:** Run all tools from the Godot project root.
* **Unified Tests:** Execute `run_tests.bat` (Builds `.dll` and runs GdUnit4 headless).
* **Macro Errors:** If Rust compiler fails inside a macro, execute `cargo expand -p <crate> --lib <mod> > expanded_debug.rs`. Read it, fix the type mismatch, and delete the file.

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
