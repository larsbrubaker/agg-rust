@echo off
setlocal enabledelayedexpansion

rem ---------------------------------------------------------------------------
rem run_demo.cmd - Build and run the interactive AGG-Rust demo in watch mode.
rem
rem The Bun dev server (demo/server.ts) rebuilds WASM + TS on file changes but
rem does NOT perform an initial build at startup, so this script makes sure the
rem prerequisites and the initial WASM/TS builds are in place first, then starts
rem the watch-mode server at http://localhost:3000.
rem
rem Safe to double-click from Explorer or run from any working directory:
rem all paths are anchored to this script's location via %~dp0.
rem ---------------------------------------------------------------------------

set "REPO_ROOT=%~dp0"
set "DEMO_DIR=%REPO_ROOT%demo"

echo === AGG-Rust demo launcher ===
echo.

rem --- Prerequisite: bun on PATH ---
where bun >nul 2>&1
if errorlevel 1 (
    echo [ERROR] 'bun' was not found on your PATH.
    echo         The demo dev server and TypeScript build require Bun.
    echo         Install it from https://bun.sh/ ^(e.g. 'winget install Oven-sh.Bun'^),
    echo         then re-run this script.
    goto :fail
)

rem --- Prerequisite: wasm-pack on PATH ---
where wasm-pack >nul 2>&1
if errorlevel 1 (
    echo [ERROR] 'wasm-pack' was not found on your PATH.
    echo         The demo builds the Rust crate to WebAssembly using wasm-pack.
    echo         Install it with 'cargo install wasm-pack' ^(needs a Rust toolchain^),
    echo         then re-run this script.
    goto :fail
)

rem --- Prerequisite: powershell (used by the build:wasm script) ---
where powershell >nul 2>&1
if errorlevel 1 (
    echo [ERROR] 'powershell' was not found on your PATH.
    echo         The demo's build:wasm step runs demo/build-wasm.ps1 via PowerShell.
    goto :fail
)

if not exist "%DEMO_DIR%\package.json" (
    echo [ERROR] Could not find the demo project at:
    echo         %DEMO_DIR%
    echo         This script must live in the repository root next to the 'demo' folder.
    goto :fail
)

pushd "%DEMO_DIR%" || goto :fail

rem --- Install demo JS dependencies if needed ---
if not exist "%DEMO_DIR%\node_modules" (
    echo [1/3] Installing demo JS dependencies ^(bun install^)...
    call bun install
    if errorlevel 1 (
        echo [ERROR] 'bun install' failed.
        popd
        goto :fail
    )
) else (
    echo [1/3] Demo JS dependencies already installed - skipping bun install.
)
echo.

rem --- Ensure an initial WASM build exists (the server only rebuilds on change) ---
rem Invoke wasm-pack directly from the repo root rather than via the package.json
rem "build:wasm" script: that script shells out to PowerShell, which fails on
rem machines whose execution policy blocks running .ps1 files. The crate path is
rem relative to the repo root, matching how server.ts rebuilds on change.
if not exist "%DEMO_DIR%\public\pkg" (
    echo [2/3] Building WASM package ^(first run^)...
    pushd "%REPO_ROOT%" || goto :fail
    call wasm-pack build demo/wasm --target web --out-dir ../public/pkg --no-typescript
    if errorlevel 1 (
        echo [ERROR] WASM build failed.
        popd
        popd
        goto :fail
    )
    popd
) else (
    echo [2/3] WASM package already present - skipping initial build:wasm.
)
echo.

rem --- Ensure an initial TypeScript bundle exists (server does not build at startup) ---
if not exist "%DEMO_DIR%\public\dist" (
    echo [3/3] Building TypeScript bundle ^(first run^)...
    call bun run build
    if errorlevel 1 (
        echo [ERROR] TypeScript build failed.
        popd
        goto :fail
    )
) else (
    echo [3/3] TypeScript bundle already present - skipping initial build.
)
echo.

echo Starting watch-mode dev server...
echo Open http://localhost:3000 in your browser.
echo Press Ctrl+C to stop.
echo.

call bun run dev
set "EXITCODE=%errorlevel%"

popd
endlocal
exit /b %EXITCODE%

:fail
echo.
echo Demo launch aborted.
echo.
pause
endlocal
exit /b 1
