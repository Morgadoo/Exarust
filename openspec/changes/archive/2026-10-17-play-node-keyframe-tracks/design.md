## Context

How the original reads and plays a node's keyframe track.

The node reader creates the node by its chunk tag, reads the shared
header and, for a node that is not an instance, the body of its kind (a
mesh's streams and topology; a light's 40 bytes, two more words for a
spot and four more under its body flags `0x30000`; a voxel light's 32
bytes; two words for kind `0x3D08`; nothing for the others). Then, for
every kind, it walks the remaining sub-chunks: `0x3AC1` keyframe track,
`0x3D0CEC05` GI points, `0x3DD0B001` rigid body, `0xEDC0C589` morph blob,
and the registered handlers for the rest (the cloth marker, the emitter
records). A track sub-chunk adds a Catmull-Rom position controller to
the node's list: it skips a word, reads the mode word, the duration
(float), the last key's index and that many plus one 16-byte keys (time,
then three floats), and points the controller's output at the node's
local translation.

Nothing plays a track by itself. The scene keeps a list of playing
tracks (node, controller, start time in ms of the frame clock). Each
frame, for every entry, it computes t = (now − start) × 0.001 s, asks the
controller's clock for the curve time and whether it goes on, lets the
controller write the position, and removes the entry when the clock says
it ended. The clock: mode 1 (once) clamps t at the duration and ends
there; mode 2 (reverse) takes duration − t and ends at 0 or below;
mode 3 (loop) subtracts the duration times the integer part of
t / duration; any other mode passes t on and never ends. The curve, for
keys of time tᵢ and position pᵢ: at or before the first key's time the
first position, at or past the last key's time the last; otherwise the
first key i at or past t and the one before it bracket it, s is t's share
of their span, and the position is the cubic Hermite sum
pᵢ₋₁·(2s³ − 3s² + 1) + pᵢ·(3s² − 2s³) + m₀·(s³ − 2s² + s) + m₁·(s³ − s²),
with m₀ = 0 on the first segment and (pᵢ − pᵢ₋₂)·(tᵢ − tᵢ₋₁)/(tᵢ − tᵢ₋₂)
after it, and m₁ = (pᵢ₊₁ − pᵢ₋₁)·(tᵢ − tᵢ₋₁)/(tᵢ₊₁ − tᵢ₋₁) before the
last segment and pᵢ − pᵢ₋₁ on it (the factors 0 and 1 are constants in
`.rdata`).

A start adds a (node, controller) pair to the list unless it is already
there, with the clock's time, and then swaps the controller's mode
between 1 and 2. The only caller starts every track of a node and of the
nodes under it, recursively; its only caller is the use of an object
whose operative section is of class 0 and whose state word allows use
(it then plays the object's sound and signals its target, at once or,
for kind 1, 2.1 s later). The player uses an object by clicking it with
the interact modifier; that path (the pick under the cursor) is outside
Exarust today.

The frame clock is the engine's ms timer less the time it is told to
skip: with a frame cap every frame adds exactly one frame interval. It
is not the 12 ms body-tick clock of the character rules.

What the data holds (corpus, every chunk tree of every pack, nested
packs included, 7 653): one track. It sits on `Box001`, a mesh under the
root `Plane001` of `Objects.rpk` `uwa01 brkswtch` (the "Secret Swt Brk"
object, operative class 0 kind 1, placed by `exanima03` from its own
database at (−40.65, 1.93, 83.29) m, rotated half a turn): mode once,
3 s, keys (0, 0 cm), (1 s, z −5 cm), (2 s, z −5 cm), (3 s, 0 cm). Both
nodes sit at the origin, so the track moves the brick 5 cm along its
parent's −z (into the wall) and back. The first start swaps once to
reverse; the keys are symmetric, so it looks the same either way.

The research notes, with addresses, are git-ignored:
`context/re/master-animation.md` (§12 and "M20 findings"),
`context/re/master-plan.md` (B9).

The format page this change extends is
[`docs/formats/RFC.md`](../../../docs/formats/RFC.md) (Scene nodes,
Keyframe tracks).

## Goals / Non-Goals

**Goals:**
- Decode every keyframe track the data holds, on any node kind, and play
  it exactly as the original's scene does once it is started, with the
  original's clock, curve, start swap and end.
- A start interface for the future use of objects, and a debug start.

**Non-Goals:**
- Using objects and the operative signals (the switch's delayed signal).
- The skipped word, `0x3D0CEC04`, the linear controller nothing builds.

## Decisions

1. **Sub-chunks after every kind's body.** The original's loop runs for
   every stored node; reading it only after meshes would hide tracks on
   other kinds. The body lengths are the original's; the corpus walk
   ending exactly at every node's end (and the nine lights with the
   extra words) confirms them. The GI points stay on meshes (Exarust
   keeps them in the mesh); a body or cloth marker on another kind would
   now be read, and none exists.
2. **Data in `exa_formats::key_track`, playing in `exa_anim::tracks`.**
   The format crate stays Bevy-free and only decodes; the curve and the
   clock are animation, beside the clip samplers. Positions are stored in
   metres (the curve is linear in the positions, so converting first
   gives the same curve).
3. **Each node entity carries its tracks** (`NodeTracks`, with the mode
   each start swaps), and a resource holds the playing list (entity,
   track index, start). The start message names a node and covers the
   nodes under it, as the original's recursive start does; a use of an
   object will send it for the object's root. `exa_world` now depends on
   `exa_anim` to insert the component when it spawns a model.
4. **The clock is Bevy's virtual time** in `Update`: real time, still
   while paused (P) and scaled by `--sim-speed`, as the original's frame
   clock skips the time it is told to; not the fixed body tick.
5. **Started and evaluated in the same frame**: the start frame
   evaluates at elapsed 0 (a reversed once track at its last key), as the
   original's next scene pass would.
6. **Robustness the original lacks:** a node that is despawned leaves the
   list; a loop with a zero duration runs on as free (the original's
   integer wrap leaves the time as it is); a track without keys (not in
   the format) writes nothing.
7. **F7 starts every track of the level**, the debug stand-in for using
   the object, beside the other debug keys; it logs each start.

## Open questions and how they were answered

| question | answer | how |
|---|---|---|
| Which node kinds read sub-chunks? | all stored nodes, after the kind's body | REA on the node reader; corpus walk |
| The light body's length | 40 B, +8 for a spot, +16 under flags `0x30000` | REA on the light reader; 9 lights in the corpus |
| Key layout and units | time (s), position (cm), 16 B | REA (keys read as 16 B, output to the node's translation); the shipped keys |
| Units of the clock | seconds of the frame clock (ms × 0.001) | REA on the scene pass and the frame timer |
| Curve slopes at the ends | 0 leaving the first key, the last chord arriving | REA: factors 0.0 and 1.0 in `.rdata` |
| Who starts a track? | the use of a class-0 operative object, recursively over its nodes | REA call chain from the player's interact pick |
| Does a start restart a playing track? | no; a new start swaps 1 ↔ 2 | REA on the start |
| How many tracks ship? | one (the secret switch's brick) | `exa-tool corpus tracks`, corpus test |
| Does the frame clock stop in the pause menu? | open: it skips the time it is told to (likely the pause); Exarust's virtual clock stops on P | REA on the frame timer; not settled by a capture |

## Risks / Trade-offs

- [The pause behaviour of the original's frame clock is inferred] → it
  only shifts when a playing track resumes; a capture of the retail game
  pausing mid-press would settle it.
- [Reading sub-chunks after every kind could misread a body of unknown
  length] → the corpus test pins one track and no stray body or cloth
  marker; the walk ends exactly at every node's end in the shipped data.
