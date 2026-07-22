cmake_minimum_required(VERSION 3.15)
project(agg_render CXX)

set(CMAKE_CXX_STANDARD 11)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

if(NOT CMAKE_BUILD_TYPE)
    set(CMAKE_BUILD_TYPE Release)
endif()

# Locate the original AGG 2.6 C++ sources shipped in the repo.
set(AGG_ROOT "${CMAKE_CURRENT_SOURCE_DIR}/../../cpp-references/agg-src")

include_directories(
    "${AGG_ROOT}/include"
    "${AGG_ROOT}/examples"
)

# Core AGG implementation files (everything under src/ except the platform/ GUI
# backends, which we replace with a headless harness).
file(GLOB AGG_CORE_SRC "${AGG_ROOT}/src/*.cpp")
file(GLOB AGG_CTRL_SRC "${AGG_ROOT}/src/ctrl/*.cpp")

# Example helper translation units reused by the demos.
set(AGG_EXAMPLE_HELPERS
    "${AGG_ROOT}/examples/parse_lion.cpp"
    "${AGG_ROOT}/examples/interactive_polygon.cpp"
)

file(GLOB RENDERER_SRC "${CMAKE_CURRENT_SOURCE_DIR}/src/*.cpp")

add_executable(agg-render
    ${RENDERER_SRC}
    ${AGG_CORE_SRC}
    ${AGG_CTRL_SRC}
    ${AGG_EXAMPLE_HELPERS}
)

# Demo data assets (shapes.txt, spheres.ppm, compositing.ppm live under art/;
# spheres.bmp used by the Rust side lives under demo/wasm/src/).
target_compile_definitions(agg-render PRIVATE
    AGG_ASSET_DIR="${AGG_ROOT}/examples/art"
    AGG_SPHERES_BMP="${CMAKE_CURRENT_SOURCE_DIR}/../../demo/wasm/src/spheres.bmp")

if(MSVC)
    target_compile_options(agg-render PRIVATE /wd4244 /wd4267 /wd4996 /EHsc)
    target_compile_definitions(agg-render PRIVATE _CRT_SECURE_NO_WARNINGS)
endif()
