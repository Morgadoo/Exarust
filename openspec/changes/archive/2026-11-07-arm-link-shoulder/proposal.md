## Why

A player holding a polearm (`--weapon IwDogHk`) crouched and fell at idle.
The arena duellist with the same weapon went to Rebalance at the same tick
and only just recovered.

## Evidence

- Per-tick grip trace of both. Both had the same moveset (config 64,
  Polearm), the same clips and the same modes. In mode 2 (Idle) the arm
  link was enabled at about 26 cm and relaxed to no slack, with 7 cm it
  could not close. It hauled the left arm out sideways; the bodies then
  entered the Defence stance, whose two-hand link was enabled at a span
  of 118 cm.
- Clip poses alone (`grip_pose_test`):
  - The Polearm moveset plays its own Idle, Defence stance, Cmbt - Idle
    and attack clips; nothing falls back to another moveset.
  - Its second-hand point lies 10–29 cm from the right hand in the combat
    clips.
  - Its arm point lies 26.6 cm from the forearm's point in Idle, but
    11.6 cm from the shoulder's (two-hander 29.0 vs 13.3 cm, greatsword
    29.8 vs 14.5 cm).
- REA:
  - The character init stores, per side, the
    torso's child with a body as the arm chain's root (`+0xe60`/`+0xe68`,
    the shoulder) and walks its first children down to the hand
    (`+0xe78`/`+0xe80`).
  - The grip link builder ties the arm link to the chain
    root of the grip's hand.
  - The shield's attach registers it under the hand's
    parent, the forearm.
  - Exarust gave a weapon's arm link the forearm.
- The grip placement (table entry, Euler order) and the clip choice
  match the original; no change there.

## What Changes

- exa_character: a held weapon's `HeldGrip.arms` are the shoulders
  (`SHOULDER_BONES`); the shield and the torch keep the forearms.
- A test pins the Idle-pose spans (shoulder under 16 cm, forearm over
  25 cm) for the polearm, the two-hander and the greatsword.
- The grip trace's per-tick holder line (clip, moveset config, condition,
  locomotion mode).
