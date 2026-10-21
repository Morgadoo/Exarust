# Tasks

- [x] Re-read the six units' instruction listings and data tables; record
      every discrepancy with its evidence.
- [x] Correct the text support rules (ellipsis, truncation, tail, line
      count, width fit, wrapped orientation, menu font) with tests pinned
      to worked numbers.
- [x] Correct the single-line editor (caret walk, window end, refused byte,
      copy and cut, paste, draw paths, tags, texture key) with tests.
- [x] Correct the main menu (page top, row step, hit band, paint, commands,
      EXPERT gate, constructor fields, resume) and the running screen's
      press and release; the menu tests pass.
- [x] Rebuild the window manager on the original's task fields (focus,
      tooltip, modal, pointer, press, release, drop, capture, reselect,
      lengths, grips, clamp, toggle switch) with tests.
- [x] Correct the viewport (jitter, samples, eye, camera scale, frame
      rate, conversions, cursor actions); check the jitter table against
      the binary.
- [x] Correct the control append, layout pass and label draw with tests.
- [x] Update `docs/UI.md`, `docs/RENDERING.md` and the READMEs.
      `docs/PLAN.md` and `docs/COMPONENT-MAP.md` are the integrator's.
- [x] No corpus format changes; nothing new reads `EXANIMA_DIR`.
- [x] Tests and clippy (`-D warnings`) for `exa_ui`, `exa_game` and
      `exarust`.
