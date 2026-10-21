# Tasks

- [x] Reproduce the record loading: gear filter, presets and keys, named
      background template, free creator, entry plan.
- [x] Reproduce the starting skills, slot clear, experience budget, worth
      and skills page.
- [x] Reproduce the kit tables, kit matching and kit item resolution.
- [x] Reproduce the voice prefix, selection, names, pitch, throttle and
      variant draw, and the hair, look and variant captions.
- [x] Reproduce the two pages, equipment panels, power tree, commits and
      back action, and the continue chooser and layout.
- [x] Review every rule against the disassembly and fix the differences,
      with tests pinned to the corrected numbers.
- [x] Corpus check: none applies, as these rules read no game files; the
      kit tables and preset keys are the executable's own data, read with
      REA, and no game bytes are in git.
- [x] Update `docs/formats/OTHER.md`, `docs/UI.md` and the crate READMEs;
      `docs/PLAN.md` is updated by the integrator.
- [x] `cargo fmt --all`, clippy with `-D warnings` and the tests of
      `exa_game`, `exa_ui` and `exa_character`.
