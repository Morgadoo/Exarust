## Why

Lane `physfix` (roadmap B4 physics): three findings of the evidence2 lane.

- F560: round 280 (the fires stepped in the simulation) changed standing
  characters far from any fire from the first tick.
- F563: exanima01's portcullises read open where they block their arches.
- F566: the hand could not take a portcullis leaf.

## Evidence

- F560, by headless traces (a new debug tool, `EXA_HEADLESS_TRACE`, runs
  the game's App without a GPU and writes the body trace): the difference
  came from the schedule's layout, not from the fires' work. Running the
  fires' two fixed systems with empty bodies traced equal to the round;
  skipping them by a run condition (one more set in the graph) or leaving
  them out changed 18 to 20 armed characters. Skipping the armour pass's
  head height made all variants equal, and ordering it first in the fixed
  step did too: it was ordered only before the grip builder, so the
  schedule decided whether the tick's stages saw the raised arm link.
- F563, in the assembly: the slider's registration and the door's set-up
  measure the fraction as (point − anchor)·axis / travel², and the stored
  anchors of the shipped portcullis (link parent points plus the leaf's
  translation) put every leaf's points on them, so the original reads
  fraction 0, its open end, at load. The sliding step slams and latches at
  fraction 1 (`slm`, the latch) and sounds `opn` leaving it, so 1 is its
  shut end. The model's rails run 170 cm along its x axis, from the leaf's
  placed pose in its arch into its wall: the original's "shut" leaves the
  arch clear, and its levers (on at load) clear a gate with their off
  signal (a close order), as players describe the shipped puzzles (a pulled
  lever opens the gate). Exarust already does this; round 274's notes had
  the placed pose in the wall. The navigation grid follows the leaf's mesh.
- F566: the press named a sliding leaf by its mobility (0.048, under the
  0.08 a prop must have) because its door lookup read hinged leaves only.

## What Changes

- The armour pass's head height runs first in the fixed step (before the
  blows are dealt and the grip builder).
- The hand's pick names a sliding leaf by its door record, as a hinged one.
- `SlideBody::at_placed_pose`; the mechanism list says where each leaf
  stands; the sliding leaves' documentation describes the rails as they lie.
- `exa-tool bodies` prints each body node's frame in its parent.
