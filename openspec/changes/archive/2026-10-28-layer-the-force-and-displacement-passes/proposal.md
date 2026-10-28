# Proposal

## Why

Phase 7 (Game modes and content) carries the thaumaturgy Force and
Displacement units (SG_Thaum_Force 47%, SG_Thaum_DIsp 45%). A DeepSeek lane
(thaumforce) added the five effect materials, their signatures and noise
picture, the four layered draw passes, the displacement shell's two render
passes, the creators' material nodes, and spawned the shell aura, strikes,
vanish, long door, summon and constant-displacement nodes in the running
game. The review read each rule in the instruction listing and found the
passes drawing a first pass with no pass count, the first draw's colour
missing, the effect material's and tendril's depth term in single instead
of double, the brightness and texture coordinate divided by the pass count
instead of the divisor, uniforms bound by passes that bind none, the aura
and shell materials' 0x9000 written to the wrong node word, the shell fade
pass keeping the fade it restores, and a Long Door moving the body a
hundred times too far (centimetres given as metres).

## What Changes

- The four material passes: nothing drawn at a pass count of zero or less;
  layer 0's colour before the first draw; the double-precision literal
  term for the effect material and tendril (`effect_geometry::force_row`
  too); brightness gated by display bit 0x40 and divided by the divisor;
  the shader alone binds the texture coordinate; the tendril binds no
  time or brightness. Four layers.
- The aura and shell materials set 0x9000 in the node's state word; the
  added sub-object's 35044 is its buffer usage. Duplicate constants
  removed.
- The shell pass: 0x200 is a depth-only pass, 0x400 the fade pass; the
  dead and thick bits are in different words; the fade pass restores the
  parts' fades.
- The noise upload runs only on a new picture.
- A Long Door places the root at the destination in metres, kept at its
  height above the body's lowest part.

## Evidence

The four material draw callbacks, the five material routines, the noise
picture's creation and upload, the shell pass and its two callers, the
shared-shell and decorative-shell creators and the body-placement routine,
read in the instruction listing.

## Out of scope

GPU submission of the passes and the original's GLSL; the effect geometry
classes' meshes; the vanish's show and hide on Exarust's drawn body; the
ground query's height offset (the lowest part stands in); sound playback;
the demon spawn.
