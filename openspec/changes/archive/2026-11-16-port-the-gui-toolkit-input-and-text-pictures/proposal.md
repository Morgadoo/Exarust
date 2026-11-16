## Why

Roadmap task B4 (lane `uitoolkit`, port lane L15 UI toolkit & input): the
window procedure's button words reached the base-form window managers only
as a plain left button, the focus loss left a held press and the capture
in place, the GUI double click used the built-in 450 ms the original
overwrites with the system's time, the confirmation dialogs ignored a
press off their buttons, the manual's `[rimg=NAME]` pictures were laid out
at size 0 and never drawn, and the default tooltip drew a flat quad.

## Evidence

REA (Exanima.exe 0.9.5), the assembly where constants and order matter:
the window procedure's button, focus and character messages; the left and
middle press routine and the right press word (Ctrl 0x40, Shift 0x20, Alt
0x80); the controls switch that hands the middle button to the controls'
hook while they are on; the GUI root's press and release; the four
confirmation dialogs' press slot (hover word not a record: close, answer
0x40); the markup draw tag's picture branch (texture size over six in
units, white ink under blend 0x103); the tooltip's draw (rounded fill at
(1, 1), one pixel short, radius word 2, alpha fade x 153).

## What changes

- Right and middle presses and releases reach the base-form window
  managers with the modifier bits; the middle only while the controls are
  off.
- Focus loss releases the held buttons, drops the capture and parks the
  focus; its return refocuses the remembered form.
- The GUI double click is 500 ms, shared with the world's.
- The arena's Dismiss and yes/no dialogs and the save screen's delete and
  reset boxes close on a press off their buttons.
- The manual's inline pictures load, lay out and draw.
- The tooltip background is the skin's rounded fill.
