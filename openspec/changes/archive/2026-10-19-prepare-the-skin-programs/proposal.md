# Proposal

## Why

A 333-byte method builds the skin's three shader programs and sets their projection uniform. Exarust does not model the selection, and the class's shader path is otherwise only known from the renderer.

## What Changes

- A guard makes the second call do nothing.
- Program 1 is a solid-colour fragment shader of kind 5; it sets `MPrj` to 2.
- Program 2 is a textured shader of kind 0x15; it sets `MPrj` to 2 and looks up `Scale`.
- Program 3 is a rounded-rect shader of kind 5; it sets `MPrj` to 2, sets `Tex` to 0, and looks up `TexCol`.
- Six uniform operations are counted.

## Capabilities

### New Capabilities

### Modified Capabilities

- `user-interface`: the skin's shader programs.

## Impact

- `exa_ui::skin`, from `prepare_skin_programs`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`.
- Evidence: the method's three source pointers, four uniform names and two constant values, all read from the binary. No new corpus files.
- Out of scope: issuing the corresponding GPU work.

## Phase

Phase 7, the interface. The program selection, not the GPU submission.
