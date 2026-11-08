## Why

PLAN Phase 6 (thaumaturgy effects): TScanEffect, TBubbleEffect and
TForceDomeEffect. The scan, the teleport bubble and their draws were ported
and reviewed, but no shipped level reached them: the single click on a
teleporter or reconstruction device answered "no" without running the
device's own use (nothing sent `UseDevice`), and the open destination
screen had no pointer input, so no row could be chosen and no journey
started. The force dome's destructor (the hum voice and the GI light) was
a record nobody set.

## Evidence

REA (Exanima.exe), checked against the assembly:

- The controls' single click runs a device's use before it decides the
  card or the narration; for the teleporter (class 2 kind 0x0F) and the
  reconstruction device (kind 0x12) that use is the device's own, which
  measures the reach from the player and scans the player.
- The scan's creator (two creation draws, 4800/800 ms, the 0.015 texture
  scale) and its node removal (unhooks the body's frame callback, removes
  and frees its light), the bubble's creator, mesh build, program and
  uniforms: already ported; this change gives them their live caller.
- The dome's destructor: a real delete first runs the class's clear hook;
  a hum voice that exists is freed; a GI light that exists is taken out
  of the scene's light list and freed; the base object leaves the scene;
  an owned object frees itself. The dome's frame at state 4 does the
  light half itself and removes the node.
- The two devices stand in exanima05 (ids 0x555 and 0x556), with no
  crystal placed on their slots.

## What changes

- `object_click`: a teleporter's or reconstruction device's single click
  sends `UseDevice` for the player's body; the click reads no answer
  (both devices are named, so the delayed card follows either way).
- `destination`: while a destination screen shows, the window's pointer
  (or the scripted pointer), the left/right press and Escape go to it as
  `PortalInput`, in physical pixels at the interface's GUI scale.
- `teleporters`: `--debug-slot FRAME` puts a stand-in crystal (A on a
  teleporter, L on a reconstruction device) on every device's slot point.
- The force barrier and bubble mark the creator's GI light and the cast's
  hum voice on the dome and log what the destructor frees.
- The scan's aura is bridged with the scan.
