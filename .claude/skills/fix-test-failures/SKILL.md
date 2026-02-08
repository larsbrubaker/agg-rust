---
name: fix-test-failures
description: "This skill should be used after running tests when failures occur. It ensures test failures are properly diagnosed through instrumentation and logging until the root cause is found and fixed. The skill treats all test failures as real bugs that must be resolved, never skipped."
---

# Fix Test Failures

This skill provides a systematic approach to diagnosing and fixing test failures. The core philosophy is that **test failures are real bugs** - they must be understood and fixed, never ignored or worked around.

## NO CHEATING - Critical Tests

**These tests validate exact behavioral matching with the C++ AGG implementation. There are no workarounds.**

Every test exists because it validates behavior that must match C++ exactly. Bypassing tests means shipping incorrect implementations.

**Forbidden actions (no exceptions):**
- Weakening assertions to make tests pass
- Changing expected values to match broken behavior
- Wrapping failing code in catch blocks to swallow errors
- Adding conditional logic to skip checks in test environments
- Commenting out assertions or test blocks
- Using `todo!()` or `unimplemented!()` to defer failures
- Relaxing precision requirements to mask numerical errors
- Mocking away the actual behavior being tested

**The only acceptable outcome is fixing the actual bug in the production code.**

## When to Use This Skill

Use this skill when:
- Tests fail and the cause isn't immediately obvious
- A test is flaky or intermittently failing
- You need to understand why a test is failing before fixing it
- You've made changes and tests are now failing

## Core Principles

1. **Test failures are real bugs** - Never skip, disable, or delete failing tests without understanding and fixing the underlying issue
2. **No cheating** - Never weaken tests, change expected values, or work around failures
3. **Instrument to understand** - Add print statements to expose internal state and execution flow
4. **Fix the root cause** - Don't patch symptoms; find and fix the actual bug
5. **Clean up after** - Remove instrumentation once the fix is verified

## Test Failure Resolution Process

### Step 1: Run Tests and Capture Failures

Run the failing test(s) to see the current error:

```bash
# Run all tests
cargo test

# Run tests in a specific module
cargo test --lib basics_tests

# Run a specific test
cargo test test_name -- --exact

# Run with output visible
cargo test test_name -- --nocapture

# Run with backtrace on failure
RUST_BACKTRACE=1 cargo test test_name -- --nocapture
```

Record the exact error message and stack trace. This is your starting point.

### Step 2: Analyze the Failure

Before adding instrumentation, understand what the test is checking:

1. Read the test code carefully
2. Identify what assertion is failing
3. Note what values were expected vs. received
4. Cross-reference with the C++ implementation in `cpp-references/agg-src/`
5. Form a hypothesis about what might be wrong

### Step 3: Add Strategic Instrumentation

Add `println!` or `dbg!` statements to expose the state at key points:

**For pixel/rendering failures:**
```rust
println!("Pixel at ({}, {}): rgba({}, {}, {}, {})", x, y, r, g, b, a);
println!("Coverage: {}, Area: {}", cover, area);
```

**For numerical precision failures:**
```rust
println!("Expected: {:.15}", expected);
println!("Actual:   {:.15}", actual);
println!("Diff:     {:.15e}", (expected - actual).abs());
```

**For function execution flow:**
```rust
println!("Entering function with args: {:?}, {:?}", arg1, arg2);
println!("Returning result: {:?}", result);
```

### Step 4: Run Instrumented Tests

Run the test again with output visible:

```bash
cargo test test_name -- --nocapture
```

### Step 5: Identify Root Cause

Based on instrumentation output, determine:
- Is the test wrong (rare)?
- Is the code under test wrong (common)?
- Is there a numerical precision issue?
- Is there a type conversion problem?
- Does the Rust implementation diverge from C++ behavior?

### Step 6: Fix the Bug

Fix the actual bug in the production code.

Common fixes for this project:
- **Blending errors**: Verify against C++ pixel blending formulas
- **Coverage issues**: Check rasterizer cell accumulation
- **Precision issues**: Check floating point order of operations
- **Type mismatches**: Ensure proper conversions between color types
- **Edge cases**: Verify handling of empty paths, degenerate geometry

### Step 7: Verify and Clean Up

1. Run the test again to confirm it passes
2. Run the full test suite to ensure no regressions: `cargo test`
3. **Remove all instrumentation print statements** - they were for debugging only
4. Commit the fix

## Iterative Debugging

If the first round of instrumentation doesn't reveal the issue:

1. Add more instrumentation at earlier points in execution
2. Log intermediate values, not just final state
3. Compare intermediate values with C++ execution
4. Check for side effects from other code
5. Verify test setup is correct

Keep iterating until the root cause is clear.

## C++ Comparison Technique

When stuck, compare execution step-by-step with C++:

1. Find the corresponding code in `cpp-references/agg-src/`
2. Add corresponding print statements in both Rust and C++ code
3. Run both with the same input
4. Find the first point of divergence
5. That divergence point is your bug location

## What NOT to Do (NO CHEATING)

- **Don't skip failing tests**
- **Don't delete tests** to make the suite pass
- **Don't use `#[ignore]`** as a permanent solution
- **Don't weaken assertions**
- **Don't change expected values** to match broken output
- **Don't use `todo!()`** to defer the fix
- **Don't relax precision** to hide numerical errors
- **Don't leave instrumentation** in committed code

**If you find yourself wanting to do any of these, STOP. The test is telling you something is broken. Fix the broken thing.**
