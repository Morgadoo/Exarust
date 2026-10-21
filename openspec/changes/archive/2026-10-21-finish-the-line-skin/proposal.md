# Proposal

## Why

The game's GUI skin (the original's `TGSLines`, a thin-line subclass of the
draw skin) was rated 60%. A lane added a second copy of its dispatcher and
helpers beside the models already in `exa_ui::skin`, rewrote the parent's
centred icon from the line override and zeroed the fifth unit length. The
review keeps one model of each method, checked against the assembly, and adds
the methods that were still missing.

## What Changes

- The parent's centred icon and the fifth unit length (four units) come back.
- The line skin gets its own centred icon, scroll refresh, progress strip,
  integer span bar and maximised window frame (`exa_ui::line_skin`).
- The dispatcher draws the maximised frame and the progress strip, and its
  close-edge tints follow the tab quads they colour.
- The hit test falls through from the title band to the strip test.
- The rail's right cap and thumb follow the assembly.
- The lane's duplicate dispatcher, shade, square, capped span, label and span
  bar are removed.

## Capabilities

### Modified Capabilities

- `user-interface`: the line skin's methods.

## Impact

- `exa_ui::skin`, `exa_ui::line_skin`, `crates/exa_ui/README.md`, `docs/UI.md`.
- Evidence: REA/Ghidra on Exanima.exe: the line skin's methods and its
  table, the parent's constructor and centred icon, checked instruction by
  instruction where the pseudo-code reorders or hides terms.
- No corpus files: these are draw rules, not file formats.

## Out of scope

- Drawing the line skin in the running game: no form submits its quads yet.
- The icon manager's and the font's draws, the clip stack and the shadow.

## Phase

Phase 7, the interface: the GUI skin.
