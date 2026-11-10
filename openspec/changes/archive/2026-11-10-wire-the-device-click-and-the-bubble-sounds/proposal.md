## Why

PLAN Phase 6 (thaumaturgy effects): TScanEffect, TBubbleEffect and
TForceDomeEffect. The scan, the teleport bubble and their draws were ported
and reviewed, but no shipped level reached them: the single click on a
teleporter or reconstruction device answered "no" without running the
device's own use (nothing sent `UseDevice`), and the open destination
screen had no pointer input, so no row could be chosen and no journey
started. The bubble played none of its sounds, its GI light swelled with
the drawn sphere, and a journey's level change carried the level portals'
flags. The force dome's destructor was a record nobody set.

## Evidence

REA (Exanima.exe), checked against the assembly:

- The controls' single click (both the node and the point-search paths)
  runs a class 2 device's use before it decides the card or the
  narration; for the teleporter (kind 0x0F) and the reconstruction device
  (kind 0x12) that use is the device's own, which measures the reach from
  the player and scans the player. Its answer is yes when the screen opens
  or is already open, or the scan starts or already runs; the point-search
  path narrates the description on a yes whatever the name.
- The bubble's creator loads `teleportf01` and `teleportc01` once, gives the
  bubble a voice for `teleportf01` at its place (0.3, pitch 1); the step
  plays `teleportc01` (0.3, pitch 1, flags 0) when a plain bubble has grown
  and writes the voice's volume `(value + 1)·0.5` while it shrinks; the end
  routine frees the voice and the light.
- The bubble light's program uploads `Radius` from the node's stored
  120 cm and its bounds use the node's box (also 120, set once); the
  charge's swell (compared and computed in double precision) scales the
  colour and pulse only.
- The journey's level change (the bubble at state 4) queues the level
  change with flags 0; the zone exits of the level portals pass 1, which
  runs the portal use (the dissolve burst and `portaluse01`), the AI
  hand-over and the long transition.
- The dome's destructor: a real delete first runs the class's clear hook;
  a hum voice that exists is freed; a GI light that exists is taken out
  of the scene's light list and freed; the base object leaves the scene;
  an owned object frees itself.
- The two devices stand in exanima05 (ids 0x555 and 0x556), with no
  crystal placed on their slots.

## What changes

- `teleporters::DeviceReads`: the device use's decision (refused, nothing,
  open the screen, scan), shared by the fixed step's use and the click.
- `object_click`: a teleporter's or reconstruction device's single click
  reads that answer at once and sends `UseDevice` for the player's body.
- `destination`: while a destination screen shows, the window's pointer
  (or the scripted pointer), the left/right press and Escape go to it as
  `PortalInput`, in physical pixels at the interface's GUI scale; the
  screen holds the pointer (`PointerOverGui`), so the world's click paths
  leave it alone.
- `effects::bubble_sound`: the bubble's spawn voice, its close sound, the
  shrink's volume and the voice's release.
- `thaum::effect_light::bubble_shape`: the light's radius and box are the
  node's 120 cm; `emissive::teleporter_light` swells in double precision.
- `flow`: a journey's level exit passes no flags; the portal bit's step is
  `Step::PortalUse` (was `FadeOut`; no executor yet).
- `teleporters`: `--debug-slot FRAME` puts a stand-in crystal (A on a
  teleporter, L on a reconstruction device) on every device's slot point.
- The force barrier and bubble mark the creator's GI light and the cast's
  hum voice on the dome and log what the destructor frees; the scan's aura
  is bridged with the scan.
