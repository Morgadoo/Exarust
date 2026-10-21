# Design

## Approach

The original's frame layer is a Win32 message loop that winit and Bevy
replace, so its rules are reproduced as pure models pinned by tests, and
only the parts with an observable effect on Exarust are wired: the
frame-rate cap (when asked for), the unfocused hold, the focus release,
the minimum window size, the full-screen rule, the present mode and
Alt+Enter. The settings' window rules stay in `exa_game::hw_config`, which
now calls `exa_game::window::fullscreen_mode`; the launch flag gives the
same borderless window `hw_config` would.

Frame time is unchanged by default: the cap is off unless `--max-fps` or
`FPSLimit` asks for it (the original's default is automatic, which under
vertical sync only sleeps until a millisecond before the next blank), and
the present mode is plain Fifo, the original's swap interval 1.

## Decisions

- Full screen is borderless unless `ScreenWidth`/`ScreenHeight` name a
  listed size other than the desktop's: the original creates a popup over
  the desktop and changes the display mode only then. Wayland has no
  primary monitor, so there it is always borderless.
- The adaptive swap interval is never chosen: nothing in the executable
  sets its switch.
- The fixed-rate path of the tick has no writer and is modelled for the
  clock only.
- The deferred ring's wrap quirk (only the head runs once the calls wrap)
  is kept in the model.
- The render-scale mode is a separate switch; at its default each
  super-sampled sub-view is the window's size.

## Open questions

- What reads the view's stored range (far − 1, else 25 000): not traced;
  answered by xrefs on the view field when the culling unit is reviewed.
- Which hook draws the frame copy refracting surfaces read: the scene pass
  does not copy; answered by tracing the hook lists' registrations (REA
  xrefs on the dispatch's list).
- What the status line's two kinds mean on screen: answered when the
  status drawing is reproduced.

## Formats

No `docs/formats/` page changes; the settings involved are in
`docs/formats/SETTINGS.md` (`FPSLimit`, `ScreenWidth`, `ScreenHeight`).
