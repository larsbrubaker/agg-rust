# Claude Code Guidelines

## Project Phase

agg-rust is a pure-Rust 2D rendering library in its own right (agg-gui and MatterCAD build on it) that began as a strict port of AGG 2.6. The project is now **beyond the port**: its goal is to **improve the library beyond the original C++ version**, with C++ behavioral matching as the baseline. A few C++ headers and demos still have no Rust counterpart; `PORTING_PLAN.md`, the open work, lists them.

The work beyond the port:

- Fix bugs, including bugs inherited from the C++ original
- Improve performance
- Extend capabilities with new APIs that don't exist in C++ AGG when they provide real-world value

C++ behavioral matching remains the default baseline. Any divergence from C++ behavior must be a **deliberate, justified fix of a demonstrable C++ bug** — never accidental drift. Every deliberate divergence must be:

- **Documented in code comments** explaining what the C++ bug was and why we diverge from it
- **Locked in with a regression test** that proves the corrected behavior and guards against reverting to the C++ bug

The model example is the gradient LUT off-by-one fix (see `seg_len` in `src/gradient_lut.rs`): upstream `agg_gradient_lut.h` sizes segments as `end - start + 1`, but the correct sizing is `end - start - 1` so gradient segments reach their end color. That divergence is documented at the call site and covered by a regression test.

**Guidelines for new extensions:**
1. Clear, demonstrated real-world use case (not speculative)
2. Tests that prove correctness — ideally a round-trip or comparison against the equivalent existing API
3. Clean Rust-idiomatic API design; document any invariants or limitations the caller must respect
4. No exposure of internal implementation details as public API — use `pub(crate)` for helpers that bridge modules

## Philosophy

**Quality through iterations** - Start with correct implementations, then improve. Code that doesn't matter can be quick and dirty. But code that matters *really* matters—treat it with respect and improve it meticulously. In a porting project, every function matters.

**Circumstances alter cases** - Use judgment. There are no rigid rules—context determines the right approach. However, this project has strong defaults because porting demands precision.

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
- Tests must verify **exact behavioral match** with the C++ implementation
- When test failures occur, use the fix-test-failures agent (`.claude/agents/fix-test-failures.md`) — it treats all failures as real bugs and resolves them through instrumentation and root cause analysis, never by weakening tests
- For pixel-perfect rendering validation, compare rendered pixel buffers byte-for-byte with C++ output

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
```

## Code Quality

**Names** - Choose carefully. Good names make code self-documenting. Rust names should follow Rust conventions (`snake_case` for functions/variables, `PascalCase` for types, `SCREAMING_SNAKE_CASE` for constants).

**Comments** - Explain *why*, not *what*. The code shows what it does; comments should reveal intent, tradeoffs, and non-obvious reasoning. When porting from C++, comments explaining *why* the Rust approach differs from C++ are especially valuable.

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

This project is a strict port of the AGG 2.6 C++ library to Rust. These rules ensure fidelity:

### Exact Behavioral Matching
- Rust implementation must match C++ behavior exactly
- Same algorithms, same mathematical precision
- Same edge case handling, same error conditions
- Same performance characteristics (or better)
- Pixel-perfect rendering output (byte-for-byte match with C++ rendered buffers)
- No "close enough" implementations

**Sanctioned exception:** known C++ AGG bugs may be deliberately fixed, diverging from the original behavior — but only following the documentation + regression-test requirements defined in the Project Phase section (code comment explaining the C++ bug, plus a regression test locking in the corrected behavior). Undocumented or accidental divergence from C++ behavior remains forbidden.

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
- "Close enough" or "good enough for now" implementations

## C++ Reference

The original C++ source code is available in `cpp-references/agg-src/` for reference. Key directories:
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
