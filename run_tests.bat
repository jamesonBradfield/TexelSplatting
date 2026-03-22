@echo off
echo [1/3] Running Strict Clippy Checks...
:: Force Aider to fail on warnings, matching Bacon's reality
cargo clippy --color always -- -D warnings
if %errorlevel% neq 0 (
    echo [ERROR] Clippy found warnings or errors. Fix these before testing.
    exit /b %errorlevel%
)

echo [2/3] Building Rust GDExtension (Workspace)...
cargo build --color always
if %errorlevel% neq 0 (
    echo [ERROR] Rust build failed.
    exit /b %errorlevel%
)

echo [3/3] Running GdUnit4 Tests (Headless + Stdout)...
godot --headless --path . --stdout --verbose -s res://addons/gdUnit4/bin/GdUnitCmdTool.gd --add res://test/ --ignoreHeadlessMode
if %errorlevel% neq 0 (
    echo [ERROR] Godot tests failed. Check the output above for stack traces.
    exit /b %errorlevel%
)

echo [SUCCESS] All tests passed!
exit /b 0
