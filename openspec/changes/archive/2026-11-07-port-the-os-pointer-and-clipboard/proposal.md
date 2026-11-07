# Proposal

## Why

docs/PLAN.md Phase 1 (the engine's platform layer), the RF_OS_Functions
gap lane. The last rating's gap: the clipboard, the screen saver and
Alt-beep suppression, the pointer warp hold and drag clip (modelled but
unwired) and work-area fitting.

Evidence (REA, Ghidra listing and assembly of `Exanima.exe`): the
clipboard put and read and their only callers (the text box's and the
single-line edit's copy and paste); the pointer read, warp and move
routines and their callers, among them the player controls' look routine,
which reads the middle button (code 4) and V (0x56) and serves both the
orbit and Cursor Look; the GUI drag clip set/clear, its apply and edge
wrap and their callers (spinner drag, editor manipulators, the camera's
free drag); the window procedure's system-command, menu-char, mouse-move
and focus branches; the Alt+Enter toggle and window creation, which set
the full-screen flag the clip clear reads; the work-area fit.

## What Changes

- The pointer warp's four-move hold is applied on Cursor Look's release
  (`exa_game::os_pointer`, `PointerWarp`, the `settle_warp` system).
- The full-screen confinement: once a capture ends, a focused full-screen
  window confines the pointer (winit's confined grab) until the focus is
  lost (`FocusConfinement`, `confine_full_screen`).
- The system clipboard for the text controls through Windows-1252
  (`exa_game::os_clipboard::SystemClipboard`, the `arboard` crate,
  opened on first use); it implements `exa_ui::text_box::Clipboard`.

## Out of scope / findings

- The clipboard's only caller reached in play is the character creator's
  name field, which belongs to another session: the creator does not ask
  for it yet.
- The drag clip and edge wrap stay a model: their callers are the
  spinner's value drag, the editor's manipulators and the camera's free
  drag, none reached by Exarust's play.
- The original's look release warps to the aim point's screen position
  when the look began over the game view; that, and the orbit's
  recentred pointer, are the player controls' (not this lane).
- Work-area fitting: winit exposes no work area and Wayland places
  windows itself. The screen-saver refusal has no winit API on Linux; on
  Windows winit refuses it in full screen and closes Alt-key menu
  lookups as the original does.

## Impact

`exa_game` (window, os_pointer, os_clipboard, player); docs/PERFORMANCE.md,
docs/PLAYER.md, docs/UI.md, exa_game README.
