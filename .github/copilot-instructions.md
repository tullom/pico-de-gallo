# Copilot Instructions

All agent guidance for this repository — Copilot Chat, Copilot CLI,
the Copilot coding agent, and every other AI coding assistant — lives
in **[`AGENTS.md`](../AGENTS.md)** at the repository root.

That file is the single source of truth. It covers:

- Project layout and the two-workspace split (host vs firmware).
- LF line-ending policy and the `dos2unix` rule.
- The full per-crate build/lint/test commands that mirror CI.
- The wire-protocol contract (postcard enum ordering is ABI),
  schema-version bumps, and the lockstep release rule.
- The full endpoint and topic catalog.
- Dependency-change ritual, pinned-dependency rationale, and
  `cargo-deny` policy.
- FFI conventions (opaque pointer, status-code stability) and
  PyO3 / maturin conventions.
- Conventional Commits + scope rules, AI-attribution trailers, and
  the no-`Signed-off-by`-from-AI rule.
- release-please workflow, tag-prefix glossary, and the
  GitHub-Actions-tag trap.
- 16 documented common gotchas, including the past-regressions log.

This stub exists so older Copilot surfaces that look only for
`.github/copilot-instructions.md` still find their guidance. When
you edit agent guidance, **edit `AGENTS.md`, not this file.**
