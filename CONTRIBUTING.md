# Contributing

Exarust is an experimental, personal project. It is open so that it can be
read, built and fixed.

## Bug reports

Include the commit, OS and GPU, the steps you took and what happened
instead. Never attach game files, extracted assets or archives of the game.

## Changes

- **Small and self-contained** (a fix, a format field, a doc correction):
  open a pull request directly.
- **Architectural** (a new crate, a change to how data flows between
  `exa_formats`, `exa_assets` and gameplay crates): open an issue first.

Before opening anything:

```bash
make check     # cargo fmt --check, clippy -D warnings, tests
```

## Game data and reverse engineering

- Never commit, attach or link original game files or anything converted
  from them.
- Format knowledge goes in `docs/formats/` with its evidence (which entries
  were parsed, what was checked).
- Do not paste decompiled code. Describe the rule or constant and implement
  it in Exarust's own terms.
