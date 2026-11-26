# Claude Code Guidelines

## Project Phase

agg-rust is a pure-Rust 2D rendering library (agg-gui and MatterCAD build on it). It began as a strict port of AGG 2.6, the C++ library by Maxim Shemanarev.

**agg-rust is now the authoritative source of this library.** C++ AGG is no longer in development, and this project no longer tries to stay compliant with it or identical to it. What the library does is defined by this repository's code and tests, not by the C++ original.

**The goal** is to be the best possible library of its kind: the best software renderer, and the most efficient implementation of the tasks it sets out to do.

What follows from that:

- **The C++ source is reference material, not a specification.** `cpp-references/agg-src/` is the best record of what an algorithm was meant to do, and the starting point when porting one of the headers `PORTING_PLAN.md` still lists. Matching its output is never a goal in itself: where the C++ behavior is wrong, unsafe, slow or awkward, the Rust library does the better thing.
- **Behavior changes on purpose, never by accident.** A change in rendered output or in what an API does is a decision: it has a reason, a comment where the reason is not obvious from the code, and a test that pins the new behavior. Unexplained drift is still a bug: users of the library rely on its output being stable.
- **Comparisons with C++ output are regression guards.** The committed `pixel-compare` reference tests and the "Byte-identical" column of the benchmark table record where the two still agree, and catch accidental changes. They are not a requirement: a deliberate change may end the agreement for a demo, and the same change then updates or retires that comparison and says why.
- **Safe Rust stays safe.** No safe public function may reach undefined behavior, whatever its arguments. Where C++ AGG leaves a precondition unchecked, the Rust API checks it and panics (`RowAccessor::row_bytes` in `src/rendering_buffer.rs` is the model). The tests that guard the unsafe code run under Miri in CI (see Testing).
- **Efficiency is measured.** A change on a hot path is timed before and after (`pixel-compare bench`, README "Benchmarks"), and a slowdown needs a reason.

The work:

- Fix bugs, including bugs inherited from the C++ original
- Improve performance
- Extend capabilities with new APIs when they provide real-world value
- Port the remaining C++ headers and demos that earn their place (`PORTING_PLAN.md` lists them)

Where the Rust behavior differs from the C++ source, say so in a comment at the code, because many readers arrive knowing AGG, and pin the behavior with a test. The model example is the gradient LUT fix (see `seg_len` in `src/gradient_lut.rs`): upstream `agg_gradient_lut.h` sizes segments as `end - start + 1`, but the correct sizing is `end - start - 1` so gradient segments reach their end color. The difference is documented at the call site and covered by a regression test.

**Guidelines for new extensions:**
1. Clear, demonstrated real-world use case (not speculative)
2. Tests that prove correctness — ideally a round-trip or comparison against the equivalent existing API
3. Clean Rust-idiomatic API design; document any invariants or limitations the caller must respect
4. No exposure of internal implementation details as public API — use `pub(crate)` for helpers that bridge modules

## Philosophy

**Quality through iterations** - Start with correct implementations, then improve. Code that doesn't matter can be quick and dirty. But code that matters *really* matters—treat it with respect and improve it meticulously. Every function in a rendering library matters.

**Circumstances alter cases** - Use judgment. There are no rigid rules—context determines the right approach. However, this project has strong defaults because rendering code demands precision.

**No stubs, no shortcuts** - Every function must be complete and production-ready. No `todo!()`, no `unimplemented!()`, no `panic!("not implemented")`, no partial implementations. If dependencies aren't ready, stop and implement them first.

## Test-First Bug Fixing (Critical Practice)

**This is the single most important practice for agent performance and reliability.**

When a bug is reported, always follow this workflow:

1. **Write a reproducing test first** - Create a test that fails, demonstrating the bug
2. **Fix the bug** - Make the minimal change needed to address the issue
3. **Verify via passing test** - The previously failing test should now pass

This approach works because:
- The failing test proves you understand the bug
- The fix is verifiable, not just "looks right"
- You can't accidentally break it again (regression protection)
- It aligns with the principle that coding is high-leverage because it's **partially verifiable**

**Do not skip the reproducing test.** Even if the fix seems obvious, the test validates your understanding and prevents regressions.

## Testing

- Tests MUST test actual production code, not copies - Never duplicate production logic in tests. Import and call the real code. Tests that verify copied code prove nothing about the actual system.
- Tests should run as fast as possible—fast tests get run more often
- Write tests for regressions and complex logic
- Avoid redundant tests that verify the same behavior
- All tests must pass before merging
- Tests verify the library's intended behavior: they are what defines it. A deliberate behavior change updates the tests that pin the old behavior in the same change
- When test failures occur, use the fix-test-failures agent (`.claude/agents/fix-test-failures.md`) — it treats all failures as real bugs and resolves them through instrumentation and root cause analysis, never by weakening tests
- The committed comparisons of rendered buffers against C++ output (`pixel-compare` reference tests) are regression guards, not a requirement (see Project Phase)
- The tests that guard unsafe code (the library's unit tests, `tests/pixfmt_row_bounds_tests.rs` and `tests/image_accessor_bounds_tests.rs`) run under Miri in CI, on a 64-bit and a 32-bit target, to catch undefined behavior an ordinary run cannot see. Miri needs nightly; the nightly's date is pinned in `.github/workflows/ci.yml` (`MIRI_TOOLCHAIN` in the `miri` job), and must be used as written
- Never pass `+toolchain` or `--toolchain` naming a toolchain that `rustup toolchain list` does not show: rustup installs it silently. Installing a toolchain is the owner's call

**Running tests:**
```bash
# Run all tests
cargo test

# Run tests for a specific module
cargo test --lib basics_tests
cargo test --lib color_tests

# Run a specific test
cargo test test_name -- --exact

# Run with output
cargo test -- --nocapture

# Run the unsafe-code guards under Miri, as CI does, on the nightly CI pins
MIRI_TOOLCHAIN=$(awk '/MIRI_TOOLCHAIN:/ {print $2; exit}' .github/workflows/ci.yml)
cargo +"$MIRI_TOOLCHAIN" miri test -p agg-rust --lib --test pixfmt_row_bounds_tests --test image_accessor_bounds_tests
cargo +"$MIRI_TOOLCHAIN" miri test -p agg-rust --lib --test pixfmt_row_bounds_tests --test image_accessor_bounds_tests --target i686-unknown-linux-gnu
```

## Code Quality

**Names** - Choose carefully. Good names make code self-documenting. Rust names should follow Rust conventions (`snake_case` for functions/variables, `PascalCase` for types, `SCREAMING_SNAKE_CASE` for constants).

**Comments** - Explain *why*, not *what*. The code shows what it does; comments should reveal intent, tradeoffs, and non-obvious reasoning. Where the Rust code differs from the C++ source, a comment saying *why* is especially valuable.

**Refactoring** - Improve code when it serves a purpose, not for aesthetics. Refactor to fix bugs, add features, or improve clarity when you're already working in that area.

## Documentation

- Documentation lives in the code: the header comment of the file or module it describes and
  the doc comments on its items, where whoever works on that code reads it. A rule that spans
  several files goes in the crate root's header (`src/lib.rs`, or the tool's `main.rs`/`lib.rs`).
  Facts and reasons go there; open work goes in `PORTING_PLAN.md`; history and "as of" status
  are left to git.
- Markdown lives only in `README.md`, `CLAUDE.md` and `PORTING_PLAN.md` at the root, license
  and notice files, `.claude/` and `.cursor/` (agent instructions), `cpp-references/` (third-party
  reference material) and `docs/BENCHMARKS.md`, the table `pixel-compare bench-compare --out`
  writes (README, "Benchmarks"). Never add a Markdown doc.
  An idea for later, written down when Lars asks for it, lives in `docs/later/<idea>.md`: it
  is not scheduled work and needs no plan entry; delete it when it is built or dropped.
- The gate `tests/markdown_placement.rs` fails on any other `.md` or `.markdown` file, untracked
  ones included.

## C++ to Rust Porting Rules

These rules apply when porting one of the C++ headers or demos that `PORTING_PLAN.md` still lists. Start from the C++ source to understand the algorithm and its edge cases, then write the best Rust version of it: safe, idiomatic and fast. The result is judged on its own correctness and tests, not on matching C++ output.

### Template-to-Trait Mapping

AGG uses heavy C++ templates. The Rust equivalents:

| C++ Pattern | Rust Approach |
|------------|---------------|
| `template<class ColorT>` pixel formats | `trait Color` + generic structs |
| `template<class VertexSource>` converters | `trait VertexSource` + generic converters |
| `template<class Scanline>` renderers | `trait Scanline` + generic renderers |
| `template<class Rasterizer>` render fns | Generic functions with trait bounds |
| Component ordering (rgba, argb, bgra) | `trait ComponentOrder` or const generics |
| Gamma functions | `trait GammaFunction` |
| `template<class T> pod_vector` | `Vec<T>` (Rust's Vec is already what pod_vector does) |

### Dependency-Ordered Implementation
Before implementing any function:
1. Read the corresponding C++ source to identify all functions called by the target function
2. Verify all dependencies are already implemented and tested in the Rust codebase
3. If any dependency is incomplete, implement dependencies first

### Forbidden Patterns
- `todo!()` or `unimplemented!()` macros
- `panic!()` for missing functionality
- Stub functions or placeholder implementations
- Implementing without dependencies ready
- Marking functions complete prematurely
- "Good enough for now" implementations

## C++ Reference

The original C++ source code is kept in `cpp-references/agg-src/` as reference material, not as a specification (see Project Phase). Key directories:
- `cpp-references/agg-src/include/` — all public header files
- `cpp-references/agg-src/src/` — implementation files
- `cpp-references/agg-src/examples/` — demo programs

**Note**: The GPC (General Polygon Clipper) in `cpp-references/agg-src/gpc/` has a non-commercial license and is **excluded** from this port. Use `scanline_boolean_algebra` for boolean operations instead.

## AGG Rendering Pipeline

The library follows a five-stage pipeline:

1. **Vertex Source** — `path_storage`, `ellipse`, `rounded_rect`, `gsv_text`
2. **Coordinate Conversion** — `conv_curve`, `conv_stroke`, `conv_dash`, `conv_transform`, `trans_affine`
3. **Scanline Rasterizer** — `rasterizer_scanline_aa` (the heart of AGG)
4. **Scanline Container** — `scanline_u8`, `scanline_p8`, `scanline_bin`
5. **Renderer** — pixel formats → `renderer_base` → `renderer_scanline_aa_solid` etc.

## Orchestration pattern

The main session (Fable 5) acts as planner and orchestrator only and should not write or edit code directly. All implementation is delegated to the `implementer` subagent, one scoped step at a time. All post-change review is delegated to the `reviewer` subagent. The main session handles only planning, architecture decisions, and synthesizing subagent results.
