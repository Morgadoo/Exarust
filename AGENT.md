# Agent guide

Read in this order: this file → `CONTEXT.md` → `docs/INDEX.md` →
`docs/PLAN.md` → `docs/WORKFLOW.md`.

## How work is done

- **Spec-driven (OpenSpec).** Non-trivial work starts as a change under
  `openspec/changes/` (`/opsx:propose`), is implemented with `/opsx:apply`,
  and is archived with `/opsx:archive`, which updates `openspec/specs/`.
  `openspec/config.yaml` holds the rules every proposal, spec, design and task
  list follows. Run `openspec validate --all --strict` before committing.
- **Reverse engineering with REA.** Questions about the original engine are
  answered by querying `Exanima.exe` through the REA MCP tools (Ghidra
  provider; the `reverse-engineer-anything` skill), plus corpus statistics
  from `cargo exa`. Setup and conventions: `docs/RE-TOOLING.md`. Snapshots
  and notes go in `context/re/`.

## What this repository is

The Exarust runtime and the documentation needed to build, use and extend
it. The research base (dumps, Ghidra projects, captures, third-party clones,
work journal) stays on the machine, under `context/`, which is ignored.

## Rules

- **No game data in git.** Not in tests, fixtures, docs or screenshots of
  extracted raw assets. Tests use synthetic bytes. Checks that need the real
  game read `EXANIMA_DIR` and skip when it is not set.
- **No retail addresses or decompiler names in code** (`FUN_…`, `DAT_…`,
  offsets into `Exanima.exe`). A standalone runtime resolves nothing against
  the retail image.
- **Names are Exarust's.** Original class names (`TMotileDynamics`, …) are
  fine in research docs. In code, name things by what they do.
- **Evidence over guesses.** A format field documented in `docs/formats/`
  states how it was established. Guesses are labelled as guesses.
- **Parsers are total.** Malformed input returns an error, never a panic.
  `exa_rpk` and `exa_formats` stay free of Bevy.
- **Update living docs** (`docs/PLAN.md`, `docs/formats/*`, crate READMEs)
  when a change alters what they describe.

## Environment

- The game path comes from `.env` (`EXANIMA_DIR`, see `.env.example`).
- `cargo exa <cmd>` runs the release `exa-tool`, the quickest way to inspect
  data.
- Bevy is pinned (workspace `Cargo.toml`). Upgrades are their own change.
