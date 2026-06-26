# AGENTS.md

This file provides guidance for agentic coding agents (AI assistants, automated tools) working in
this repository.

---

## Project Overview

`jefscad` is an early-stage personal Rust project implementing a solid modeler. The repository
currently contains one crate:

- **`flint/`** — Rounded floating-point interval arithmetic library. Provides `Flint<T>` (a pair
  of lower/upper `f32`/`f64` bounds) and associated types (`FlintRef`, `FlintArray`, `FlintVec`,
  `FlintSlice`). Many types and operations are planned but not yet implemented.
- **`kb/`** — Personal knowledge-base in Markdown; not source code, do not modify.

---

## Critical: Nightly Rust Required

The `flint` crate uses two nightly-only features and **will not compile on stable Rust**:

```toml
#![feature(portable_simd)]      # std::simd for vectorised next_up/next_down
#![feature(macro_metavar_expr)] # ${count(...)}, ${index()} in declarative macros
```

**Always prefix Cargo commands with `+nightly`** when there is no `rust-toolchain.toml` present.

---

## Build, Check, Lint, and Test Commands

All commands should be run from the `flint/` directory, or use `--manifest-path flint/Cargo.toml`
from the repo root.

### Build

```bash
cargo +nightly build
```

### Check (fast, no linking)

```bash
cargo +nightly check
```

### Lint

```bash
cargo +nightly clippy
```

### Run all tests

```bash
cargo +nightly test
```

### Run a single test by name

Pass a substring of the test path; Cargo will run every test whose full name contains it:

```bash
cargo +nightly test <substring>

# Examples:
cargo +nightly test f32_nu_nd                  # next_up_down::test::f32_nu_nd
cargo +nightly test test_equality_flint        # cmp::test::test_equality_flint
cargo +nightly test test_from_float            # conv::test::test_from_float
cargo +nightly test fmt                        # all fmt tests
```

### Run all tests in a module

```bash
cargo +nightly test cmp::
cargo +nightly test conv::
cargo +nightly test next_up_down::
cargo +nightly test fmt::
```

### Build documentation

```bash
cargo +nightly doc --open
```

---

## Repository Layout

```
flint/
├── Cargo.toml          # Edition 2024; deps: num-traits, ryu
├── src/
│   ├── lib.rs          # Crate root: type definitions, feature flags, top-level tests
│   ├── next_up_down.rs # NextUpDown trait: scalar, SIMD, array, Vec impls
│   ├── fmt.rs          # Display/Debug formatting for Flint scalars
│   ├── conv.rs         # From/Into conversions + creation macros
│   └── cmp.rs          # PartialEq / PartialOrd implementations
kb/                     # Personal knowledge-base (Markdown) — do not modify
notes.md                # Top-level project TODO — checkbox style [x]/[_]
flint/notes.md          # Crate-level TODO
```

---

## Code Style

### Formatting

- Rust edition **2024**.
- Use `rustfmt` defaults (no `rustfmt.toml` exists); run `cargo +nightly fmt` before committing.
- 4-space indentation; no tabs.
- Trailing commas in multi-line struct literals, function arguments, and match arms.
- `where` clauses on their own line when trait bounds are complex or numerous.

### Comments

- Use `///` doc-comments for all public items (types, traits, methods, macros, constants).
- Use `//` line comments for inline notes; avoid `/* */` block comments.
- Section dividers inside large `impl` blocks use `// ----` dash lines.

### Imports

- Order: `std` / core imports first, then external crates, then `crate::` intra-crate imports.
- Use `use std::simd::prelude::*;` for SIMD imports; import additional SIMD items individually
  (e.g., `use std::simd::Select;`).
- Each module's test submodule brings parent items in scope with `use super::*;`.
- External crates (`num_traits`, `ryu`) are imported directly without aliasing.
- Use `#[allow(unused_imports)]` only when items are required solely for macro expansion.

---

## Naming Conventions

| Construct | Convention | Examples |
|-----------|-----------|---------|
| Types / Structs | `PascalCase` | `Flint`, `FlintRef`, `FlintArray`, `FlintVec` |
| Traits | `PascalCase` | `NextUpDown`, `GenNumConsts`, `FloatStringParts` |
| Methods / Functions | `snake_case` | `next_up`, `as_ref`, `to_owned`, `from_str_parts` |
| Constants | `SCREAMING_SNAKE_CASE` | `F32_SIGN_MASK`, `MAX_F32_EXACT_INT` |
| Macros | `snake_case!` | `flint32!`, `flint64!`, `impl_small_int!` |
| Type parameters | Single uppercase letter | `T` (float type), `N` (array lane count) |
| Lifetimes | `'a`, `'b` | `FlintRef<'a, T>` |
| Struct fields | `snake_case`, private | `lb` (lower bound), `ub` (upper bound) |
| Test modules | `mod test` (preferred) or `mod tests` | — |

Domain abbreviations `lb` (lower bound) and `ub` (upper bound) are canonical — use them
consistently rather than spelling out the full words.

---

## Error Handling

- This is a **pure numeric library**; the public API is infallible or panics on programmer error.
- Use `.expect("descriptive message")` inside formatting/internal helpers where failure indicates a
  bug, not a runtime condition.
- Use `unsafe { std::str::from_utf8_unchecked(...) }` only for known-ASCII buffers, and add a
  comment explaining why the invariant holds.
- For `PartialOrd` implementations, return `None` from `partial_cmp` whenever either operand
  contains a `NaN` bound — never silently discard NaN information.
- No `Result` types in the public API currently. When adding fallible operations in the future,
  prefer returning `Option<T>` for domain-level "not representable" cases and `Result<T, E>` for
  I/O or parsing.

---

## Testing

- All tests live as inline `#[cfg(test)] mod test { ... }` (or `mod tests`) blocks at the bottom
  of each source file. There are no separate integration test files yet.
- Test module name is `test` (singular) in most modules; `tests` (plural) appears in `lib.rs` and
  `fmt.rs`. Prefer `mod test` for consistency in new files.
- Tests use plain `assert_eq!` / `assert!` / `assert_ne!`; no external test framework.
- Each test function should test one logical behaviour; name it `test_<what_is_being_tested>`.

---

## Patterns and Conventions

### Generic float types

Functions and `impl` blocks that are generic over the float type `T` typically require:

```rust
where T: num_traits::Float + Copy + ...
```

Keep trait bounds minimal — only add what the body actually uses.

### SIMD

SIMD impls target `std::simd` (portable SIMD, nightly only). Lane counts `N` are always `const`
generic. Prefer `SIMDf32xN = Simd<f32, N>` style local type aliases for readability within an
`impl` block.

### Macros

Declarative macros (`macro_rules!`) are preferred over proc-macros for repetitive `impl` blocks
(e.g., `impl_small_int!`, `impl_large_int!`, `impl_partial_cmp!`). Use `${count(...)}` and
`${index()}` meta-variable expressions (nightly `macro_metavar_expr` feature) to avoid manual
indexing.

### TODO tracking

Use the checkbox notation found in `notes.md` and `flint/notes.md`:

```
[x] completed task
[_] pending task
```

Do not create separate GitHub issues or PR descriptions for in-progress work; update the relevant
`notes.md` file instead.

---

## Things to Avoid

- Do not run `cargo build` or `cargo test` without `+nightly` — it will fail.
- Do not modify files under `kb/`; it is a read-only knowledge base.
- Do not add `unwrap()` in library code visible to users; use `expect("reason")` or return
  `Option`/`Result`.
- Do not introduce `std::process::exit` or panics in the public API.
- Do not add dependencies without checking that they support `no_std` if that becomes a goal
  (tracked in `flint/notes.md`).
