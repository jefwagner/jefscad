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
