# Spec-driven workflow (OpenSpec)

Exarust uses [OpenSpec](https://openspec.dev/) (`@fission-ai/openspec`,
schema `spec-driven`). Specs describe behaviour; `docs/PLAN.md` describes
the roadmap; changes connect the two.

```
openspec/
  config.yaml          project context and rules every artifact follows
  specs/<capability>/  living behaviour contracts of what exists today
  changes/<name>/      in-flight work: proposal → specs (deltas) → design → tasks
  changes/archive/     completed changes
```

Current capabilities: `game-installation`, `pack-archives`,
`format-detection`, `texture-decoding`, `asset-inspection-cli`.

## The loop

| step | Claude Code | Codex | OpenCode | what happens |
|---|---|---|---|---|
| explore | `/opsx:explore` | `$openspec-explore` | `/opsx-explore` | think through a problem, read specs and code |
| propose | `/opsx:propose "<idea>"` | `$openspec-propose` | `/opsx-propose` | creates `changes/<name>/` with proposal, delta specs, design, tasks |
| apply | `/opsx:apply` | `$openspec-apply-change` | `/opsx-apply` | implements tasks and ticks them off |
| update | `/opsx:update` | `$openspec-update-change` | `/opsx-update` | revises the plan of an in-flight change |
| sync | `/opsx:sync` | `$openspec-sync-specs` | `/opsx-sync` | merges delta specs into `specs/` without archiving |
| archive | `/opsx:archive` | `$openspec-archive-change` | `/opsx-archive` | merges deltas into `specs/`, moves the change to `archive/` |

CLI: `openspec list`, `openspec list --specs`, `openspec show <item>`,
`openspec status --change <name>`, `openspec validate --all --strict`.

## Project rules (from `openspec/config.yaml`)

- Every change names its `docs/PLAN.md` phase and milestone, its evidence
  sources, and what is out of scope.
- Format requirements state the corpus they hold for. Tasks include a check
  against the real installation (`cargo exa …`, skipped without
  `EXANIMA_DIR`) and end with `make check`.
- Reverse-engineering questions are answered with REA (see
  [`RE-TOOLING.md`](RE-TOOLING.md)), never by guessing. Findings go in
  `context/`, and conclusions go in `docs/formats/` and specs.
- Archiving a change updates the phase status in `docs/PLAN.md`.

## First change

`openspec/changes/add-static-mesh-decoding`: Phase 1, static mesh decoding
plus `exa-tool export-gltf` and `exa-tool corpus meshes`. Start it with
`/opsx:apply add-static-mesh-decoding`.

## Setup on a new machine

```bash
npm install -g @fission-ai/openspec@latest   # Node ≥ 20
openspec update                              # refresh generated skills/commands after upgrading
```

The generated agent files (`.claude/`, `.agents/`, `.opencode/`) are tracked
so every clone gets the same commands.
