# Design

## Levels

`exa_formats::level::is_level(bytes)`: a `ChunkTree` whose top-level chunks
include the tile grid (`0x3DED`) or placed props (`0x3DE1_0000`), reading
only chunk headers. At startup the panel's `LevelList` keeps the sorted
`.rfc` stems of `Resource.rpk` that pass.

## Restart

`restart_args(args, view, level)` rebuilds the command line: it drops each
flag in a fixed list with its value (the place- and script-bound ones) and
replaces the positional level after `play` or `level` with the new one. On
Unix `CommandExt::exec` replaces the process (it keeps its PID, so a
wrapper such as `gpu_run.sh` still waits on it); elsewhere the new process
is spawned and the app exits.

## Panel

Bevy UI, right edge, toggled by F9 (Escape also closes). Rows: the current
map, four switch buttons labelled with their state, and a scrolling list of
level buttons (the current one marked). Buttons act on
`Interaction::Pressed`. The switches write `PointShadows`, `RoomView::all`,
`NavOverlay::on` and `BodyOverlay::on`.

## Input

`exa_ui::PanelPointer(bool)` mirrors the panel being open. The cursor draw
computes `pointer_visibility(shown, captured, panel)`: the world cursor only
when shown and neither captured nor under a panel; the system pointer only
when the world cursor is not drawn and the mouse is not captured. A system
after `UiSystems::Focus` in `PreUpdate` scrolls the hovered list by the
wheel, then clears the wheel and resets the mouse buttons while the panel
is open, so neither the chase camera nor the player sees them.

## Tests

`is_level` on synthetic chunk trees; `restart_args` keeps flags, drops the
place-bound ones with their values and swaps the level; the pointer rule's
four cases.
