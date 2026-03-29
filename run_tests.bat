@echo off
echo [1/3] Running Strict Clippy Checks...
:: Added --quiet to suppress standard compilation progress
cargo clippy --quiet --color always -- -D warnings
if %errorlevel% neq 0 (
    echo [ERROR] Clippy found warnings or errors.
    exit /b %errorlevel%
)

echo [2/3] Building Rust GDExtension...
:: Added --quiet to prevent printing the list of compiling crates
cargo build --quiet --color always
if %errorlevel% neq 0 (
    echo [ERROR] Rust build failed.
    exit /b %errorlevel%
)

echo [3/3] Running GdUnit4 Tests...
echo Running tests (Output suppressed to save LLM context, saved to test_run.log)...

:: Piped all standard output to a log file.
godot --headless --path . -s res://addons/gdUnit4/bin/GdUnitCmdTool.gd --add res://test/ --ignoreHeadlessMode > test_run.log 2>&1
if %errorlevel% neq 0 (
    echo [ERROR] Tests failed!
    echo.
    echo --- TEST FAILURE LOG EXTRACT ---
    :: Only print the final 50 lines to the LLM so it sees the exact GdUnit4 failure
    powershell -Command "Get-Content test_run.log -Tail 50"
    echo --------------------------------
    exit /b 1
)

echo [SUCCESS] All tests passed!
exit /b 0
