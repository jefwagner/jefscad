# backlog

Raw ideas and mid-session discoveries. Never directly worked — triaged
(promoted to `TODO.md`, merged, or deleted) during planning sessions.
See `AGENTS.md` for the two-file planning model.

## From the 0-c plan (out-of-scope items, `notes/0-c-plan.md`)

- [ ] Generation-handle IDs / within-Context abandonment — revisit if Phase 2+
      booleans demand it
- [ ] `bool_ops.rs` + `mesher.rs` *algorithm* rewrites (0-c is schema migration
      only) — includes the lateral-surface / face-with-holes cap-meshing gap
      noted in CHANGELOG §0-b
- [ ] Python `tolerance` getter/setter exposure (post-0-c; leave a code TODO)
- [ ] Exact predicates (future)

## From ROADMAP / CHANGELOG

- [ ] Extend rotate surface to bezier paths (completeness, not dice-critical —
      could slip if time-pressured)
- [ ] Lift `ExtrusionError::MultiContourNotSupported` into real multi-solid
      output — unblocked by the `NodeBRep` → `SolidSet` rename (0-c first task)

## Bugs found while building the golden b-rep safety net

- [ ] `contour_signed_area` (`geom.rs`) over-reports a cubic-Bezier contour's
      signed area by exactly 2×. The exact `∫(x·y′−y·x′)dt` pair coefficients
      are `(3/10, 3/20, 1/20, 3/20, 3/20, 3/10)`; the code uses
      `(3/5, 3/10, 1/10, 3/10, 3/10, 3/5)`. Verified via exact rational
      arithmetic and the antisymmetry identity `α_ij + α_ji = [B_i B_j]_0¹`.
      The quadratic branch is correct. Sign is preserved, so simple
      outer/hole winding still works, but nesting-depth classification for
      cubic contours is wrong for near-degenerate cases. Not dice-critical
      (glyph profiles are quadratic). Fix with a focused regression test
      against the exact rational values — the golden dumps don't include area,
      so they cannot catch this.
