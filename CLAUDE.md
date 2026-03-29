# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

`jefscad` is an early-stage Rust solid modeler. The two crates are:
- **`flint/`** — Rounded floating-point interval arithmetic library (active development). Core types: `Flint<T>`, `FlintRef<'a,T>`, `FlintArray<T,N>`, `FlintVec<T>`, `FlintView<'a,T>`, all generic over `T = f32 | f64`.
- **`jefscad/`** — Top-level application crate (stub; awaits `flint` foundation).
- **`kb/`** — Personal knowledge-base symlink; **do not modify**.

## Nightly Rust Required

`flint` uses `#![feature(portable_simd)]` and `#![feature(macro_metavar_expr)]`. **Always prefix Cargo commands with `+nightly`**; there is no `rust-toolchain.toml`.

## Commands

Run from repo root or `flint/`:

```bash
cargo +nightly build
cargo +nightly check
cargo +nightly clippy
cargo +nightly fmt
cargo +nightly test                        # all tests
cargo +nightly test <substring>            # e.g. `cmp::` or `test_from_float`
cargo +nightly doc --open
```

## Architecture

### `flint/src/`
| File | Responsibility |
|------|---------------|
| `lib.rs` | Type definitions (`Flint`, `FlintRef`, `FlintArray`, `FlintVec`, `FlintView`), SIMD helpers |
| `next_up_down.rs` | `NextUpDown` trait — next representable float (scalar + SIMD) |
| `conv.rs` | `From`/`Into` from integers/floats; `flint32!` / `flint64!` macros |
| `cmp.rs` | `PartialEq` / `PartialOrd` — intervals overlap ⇒ equal (non-transitive) |
| `fmt.rs` | `Display` / `Debug` for all five types |

Key invariant: after every operation the bounds are rounded outward by 1 ULP so the exact result always lies within `[lb, ub]`.

### Naming
- Fields: `lb` (lower bound), `ub` (upper bound) — never spell out.
- Macros: `snake_case!` (`flint32!`, `impl_small_int!`).
- Test modules: `mod test` (singular) preferred; `use super::*;` inside.

## Code Conventions

- Trait bounds: keep minimal — only what the body uses. Generic float code: `where T: num_traits::Float + Copy + ...`
- SIMD: use `std::simd` (portable SIMD). Lane count `N` is `const` generic. Use local type alias `type SIMDf32xN = Simd<f32, N>` for readability.
- Macros: prefer `macro_rules!` with `${count(...)}` / `${index()}` over proc-macros for repetitive `impl` blocks.
- No `unwrap()` in library code; use `.expect("reason")`. No `Result` in public API yet — use `Option<T>` for domain-level non-representable cases.
- `NaN` in `PartialOrd`: always return `None` from `partial_cmp` when either bound is NaN.

## TODO Tracking

Use checkbox notation in `notes.md` (repo root) and `flint/notes.md`:
```
[x] completed task
[_] pending task
```
Do not open GitHub issues for in-progress work — update the relevant `notes.md` instead.
