## Why

The user's two shots of the original's player show the torch in a compact
carry before the chest: the upper arm tucked, the elbow at 90–110°, the
flame at head height. Exarust's exploring player holds it up beside the
head. Both games bend the arm alike: the shots are of combat mode (the red
combat cursor ring; the user's game reads combat mode, "Torch" Cmbt - Idle
with the Defence stance kept for over 30 minutes), and exploring the
original plays "Torch" Idle, as Exarust does. Grip, overlay, muscles and
weight measured equal (design).

One rule differs on the way to that pose: the original's exploration
steering starts no clip but the walk, so leaving combat mode in Cmbt - Idle
keeps the carry until the player walks; Exarust's started Idle on every
standing tick.

Phase 4 (Combat and items), milestone M4 "first duel": a follow-up of
`take-the-combat-stance` (L2).

## What Changes

- The exploration steering starts only the walk, run and backpedal (from
  any looping clip) and ends them, and Exarust's interim combat walk, in
  Idle when the player stands. It starts no other clip: the stance does,
  through the set-stance helper (Idle at once when the old stance's clip
  plays) and the body tick's posture switch, which never leaves
  Cmbt - Idle. So:
  - Tab off in Cmbt - Idle with the Defence stance (3 s after an attack,
    or the target gone) keeps Cmbt - Idle, standing and turning in place,
    until the player walks; the walk then ends in Idle;
  - Tab off in the guard, or in Cmbt - Idle held as the stance, starts
    Idle at once, as before.
- Tests pin the torch arm to the installed clips on the body model:
  "Torch" defines Walk, Cmbt - Idle, Attack Mid and Idle over "One
  Handed" (whole clips; the guard is One Handed's), Idle's elbow 122.6°,
  upper arm 39.8° out, 11.4° forward, wrist (−16.4, 1.4, 18.7) cm from the
  shoulder; Cmbt - Idle's 101.5°, 24.9°, 31.8°, (−8.6, −3.0, 31.6) cm.
- Docs: PLAYER.md (Movement; The combat stance; The carried torch: the
  two carries with their numbers, which one the reference shows, what is
  not yet there), formats/RSQ.md (Motion set: overlays replace whole
  clips, the per-bone word is no mask), PLAN.md (Phase 4).

Evidence:
- REA on `Exanima.exe` 0.9.5: the exploration steering's call graph
  reaches the clip start only through the set-stance helper; its three
  indirect calls are line-of-sight queries; the Tab handler starts no
  clip; the body tick's Cmbt - Idle branch skips the posture switch
  (`context/re/combat-stances.md` §2, §14); the motion constructor copies
  each clip an overlay defines whole.
- Live retail, read-only (`/proc/<pid>/mem`, nothing written, no input):
  combat mode on, Cmbt - Idle with the Defence stance for over 30 minutes
  at the exanima01 start; the grip (−12, −1.5, 6) cm at (70°, −15°, 20°)
  to 0.04°; arm stiffness 0.4 and part factor 0.5 in Cmbt - Idle; the
  arms within 0.5–0.8° of the clip; the combat cursor colour.
- Corpus: `humanoid.rsq` clips on `humanbase.rfc` offsets (the new test);
  the per-bone words of every key.
- Exarust in game through `gpu_run.sh` (light runs, before and after,
  with the joint trace): the torch player standing, walking, in combat
  mode and leaving it, and with the torch beside a sword, in exanima04 (a
  story level, so the torch is carried; exanima01 ran out of GPU memory
  even with 3 GB free while the desktop held the rest of the card); the
  arena stances with a sword and unarmed.

Out of scope:
- The original's torso twist and head look toward the cursor (about 8° at
  the belly, 15° at the chest, 19° at the head in Cmbt - Idle), which
  leaves the original's arm about 7° off the clip: M8, after its research
  (RB20).
- The hand: the original's lies open under the torch; Exarust's skin
  curls the fingers (a skinning or hand-mesh question of its own).
- The walk's loose arms (stiffness 0.2): Exarust's "Torch" Walk is 2.4–3.6°
  off the clip on average, 9–12° at most; it waits for a live comparison.
- The grip, the "Torch" overlay, the link muscles and arm states, and the
  torch's lack of a physics body: measured equal, unchanged.

## Capabilities

### New Capabilities
(none)

### Modified Capabilities
- `player-movement`: the steering's clips (only the gaits); the carried
  torch's two carries.
- `combat`: the player's stance when combat mode turns off.

## Impact

- `exa_game` (`player`: the exploration steering's clip choice;
  `torch`: the clip tests)
- `docs/PLAYER.md`, `docs/formats/RSQ.md`, `docs/PLAN.md`,
  `crates/exa_game/README.md`
