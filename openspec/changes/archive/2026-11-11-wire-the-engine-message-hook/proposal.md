## Why

Phase 8 (performance and settings) in docs/PLAN.md, RF_Main. The main unit's
message hook (the logger every engine layer calls) was modelled in
`exa_game::frame` (presentation levels, overlay ring and layout, message
boxes) but nothing ran it: Exarust's few reported failures went to its own
log as warnings, no log file was written and the timed overlay was not drawn.
The unit's initialisation and finalisation, listed as having no callers,
are run by the run-time library's unit table.

## What the original does

REA and `exe.asm` of the main unit's routines, its initialisation and
finalisation (entries of the unit init/final table, so both run at start-up
and exit), the hook, the overlay push, expiry and draw, the status-line
push, the log-path builder, the whole-file writer, the soft box fill and the
GUI level table:

- **Initialisation** creates the log buffer, the frame buffer and the timer
  table, names a first log path, installs the hook in the logger's slot and
  starts the clocks. **Finalisation** frees them. Neither is reachable
  otherwise.
- **The hook** appends `title: text` and CR LF to the session's buffer and,
  unless a start-up flag holds it (set while the system information block
  is logged) or the level is 5, recreates the log file and writes the whole
  buffer. Levels 1–3 push the status history, 4 and 5 the overlay ring, 6
  and 7 a message box. The log path is the user folder's `<project>.log`.
- **The status history** is written but no routine reads it: nothing draws
  a status line in 0.9.5.
- **The overlay** is expired once a tick before the view is drawn and drawn
  after the interface: the soft rounded box (style 3, alpha 0x80) and each
  line wrapped at 80 units in the GUI level's main font and the skin's
  eighth label ink.
- The quad-buffer stereo eye passes of the frame tick and the 3D view are
  unreachable: the stereo mode is only ever compared and cleared.

## What Changes

- `exa_game::engine_log`: the hook (`log`), its buffer and whole-file
  rewrite, the status history, the overlay timed by the app's real clock and
  drawn through the GUI's draw queue before `render_frames`, the message-box
  handler slot, `EXA_LOG_PROBE` (a debug message on a chosen frame).
- The script reports, the display-mode refusal and the save writer's package
  failure log through the hook; `exarust` names the log file
  (`<user folder>/<project>.log`, not with `--no-settings`) and shows the
  hook's boxes through `message_box`.
- `exa_ui::gui_draw::render_frames` is public so late submitters can order
  before it.

## Capabilities

### Modified Capabilities

- `frame-loop`: the message hook, its log file and the drawn overlay.

## Impact

- Code: `exa_game::{engine_log, game_event_live, hw_config, script_cast,
  slots}`, `exa_ui::gui_draw`, `exarust::{main, message_box}`.
- Docs: `docs/PERFORMANCE.md` (Frame pacing), `docs/DEBUGGING.md`, the
  three crate READMEs.
- No simulation change: the hook and overlay are frame-side only.
