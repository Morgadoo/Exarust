## Why

Roadmap task B4, lane `screens3`: the port queue's lanes Q08 (item GUI and
character view) and Q09 (game screens and settings), player-visible rows
first. The first run never measured the GPU or stored what it found; the
benchmark ran under an open Settings form with no word of what it was
doing; a shop item tried on in the trade window could be dragged off the
preview into the pack or a basket; the arena loadout's preview let worn
apparel be dragged off it; and the Company Status window had no emblem.

## Evidence

REA (Exanima.exe 0.9.5), the assembly read where order and constants
matter: the game unit's start-up (the first-run flag, the detection called
with its Apply flag), the auto-detection and the auto quality, Apply with
no form; the GPU benchmark's head (the settings form hidden, the
"Configuration" message box with flags 0x60) and its end (the box closed),
the Defaults button (the form shown again); the character view's drag
start, its pick (the held slot), the trade window's constructor (the lock
byte) and the loadout form's constructor (the frozen byte on its character
view); the status form's paint slot and the emblem draw.

## What Changes

- The first run (FirstRun still set) runs the auto-detection and writes the
  detected levels into the settings file with no form open.
- While the GPU benchmark runs the Settings form is hidden and the
  "Configuration" box ("Analysing system" and the ellipsis) shows.
- The trade window's preview is the locked character view: a shop item
  dragged off it carries a payload no panel takes.
- The loadout form's preview is the frozen character view: only a held item
  starts a drag off it.
- The Company Status form draws the company's emblem at its top centre.

## Impact

Pictures: the Company Status window (its emblem) and the benchmark's box;
no golden scene opens either. Traces: none of these run in the simulation
step.
