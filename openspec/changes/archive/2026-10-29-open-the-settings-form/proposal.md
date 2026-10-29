## Why

The menu's SETTINGS rows only logged "not wired": the hardware configuration's model, the key-read screen and the shader warm-up screen existed, but nothing opened or drew the original's Settings form, so none of it was reachable in the running game.

Phase 4 (interface) in docs/PLAN.md: the original's settings form, reachable from the main menu.

## What Changes

- The form's three record arrays are read whole (classes, ids, 16-bit places with anchors, radio groups, styles, states, step counts, callbacks), correcting the volume slider's class, the buttons' gap anchor and the style words; they are laid out by the GUI's control layout pass at unit 4 and rescaled with the form's two pages by its rescale, and the form is centred.
- A form state routes the pointer through the control classes' messages (press, drag, release, hover, set) and turns their reports into the callbacks' events; the page switch, the wheel's scroll of the Controls page and the super-sampling slider's reset of the anti-aliasing slider follow the original.
- The running game opens the form from the menu's SETTINGS rows, draws it through the GUI draw path, routes its events to the hardware configuration and the binding editor, closes it with the menu, and draws the key-read and warm-up screens' text.
- Apply's bindings are saved to the settings store at once; the exit save keeps them.
- `--open-settings settings|controls` opens the form for scripted runs.

## Evidence

The Settings constructor, the page switch, the keyboard/controller view switch, the slider callbacks, the opening's row fill, Cancel and the menu's open and hide routines, read in the hwconfig pack and the full disassembly; the record arrays' bytes read from the executable; the form show routine's centring and rescale.

## Out of scope

The controller view and its help text box (Steam Input), dragging the scrollbar's thumb, the check box's tick glyph and skin draw, the video-memory figure.
