# Proposal

## Why

The GUI core round (Phase 7 — Game modes and content, the main menu and
the GUI units behind it, see docs/PLAN.md) closed the remaining rules of
the window manager, the 3D viewport control and the main menu: the
message-box callbacks, the form factory table, the viewport's delegates
and draw slot, the main menu's show paths, its title art and its select
and click cues. Review against the assembly then corrected several of
those rules: the sample shares had been moved to single precision, the
release and hit delegates and the draw slot did not follow the original,
the no-form cursor and the double click's clock difference had been
changed the wrong way, a visibility helper repeated the existing toggle
switch line for line, and the menu cues used a guessed gain, an invented
generator and a direct Bevy player.

Evidence: the pack's assembly of the GUI task's show/hide helper, its
hit-and-focus step, its press routine, the three message-box callbacks and
the box's construction, the viewport's begin, press, release, key,
redraw, hit, shape and extra-key callbacks, its whole-frame sampler and
draw slot, the main menu's row scale, paint, select, confirm and key
handlers (the audio environment's one-shot call with gain 0.06 in XMM3, the
pitch on the stack and flags 0x4060), and the constructor's sound and
texture names.

## What Changes

- `exa_ui::gui_core`: `set_control_on` answers the state's low byte
  without the lit bit (the duplicate `set_control_visible` is removed);
  hit-and-focus on no form stores cursor 0; the double click compares a
  signed 64-bit clock difference; `MESSAGE_OK`, `MESSAGE_CANCEL`,
  `MESSAGE_CLOSE`, `message_box_button`; `TooltipClasses::clear`.
- `exa_ui::viewport`: double-precision shares; `ViewDelegates` with the
  press, release, key, begin, redraw, hit, shape and extra-key routes;
  `ViewportRegistry`, `ViewSlot`; `plan_draw` (`DrawInput`, `DrawPlan`)
  with the edge test, counter hold, count limit, step messages and
  whole-frame stereo shift.
- `exa_ui::menu`: `MENU_SOUND_GAIN` 0.06, `MENU_SOUND_FLAGS` 0x4060,
  `menu_sound_pitch` from a double draw, `title_place`, `MENU_TITLE_INK`,
  `MENU_TITLE_TEXTURE`, `menu_show` and `MenuOpen`; the row scale's
  reciprocal literal.
- `exa_game::menu_screen`: draws `maintitle01.rfi`; plays `MenuSel` and
  `MenuClick` as audio-environment one-shots with draws from the run-time
  library's Mersenne twister. `flow::hover_sound` and `menu_pages::row_h`
  reuse the menu helpers.
- `exa_ui::gui_text`: the centred line's float order; the menu font's
  sample step.

## Out of scope

- `TGUIForm`, `SG_GameGUI` and `SG_GameMain` (another lane).
- The tooltip form's own drawing; the hosted view object's GL passes and
  stereo rendering (`exa_render`).
- The Win32 pointer capture and the OS clipboard (platform glue).
- The session work behind the main menu's commands (game modes).
- The viewport's draw-slot plan, delegates and registry have no caller in
  the running game, which draws its world through `exa_render`.
