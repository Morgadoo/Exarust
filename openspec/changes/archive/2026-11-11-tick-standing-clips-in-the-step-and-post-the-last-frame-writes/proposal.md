## Why

Round 197's sweep left three frame writers that undo a free-running simulation's newer steps. The frame advanced the clip player of every body without a body tick (`play_clips`) and sent the whole animator back. The step also writes those animators (starts, requests, captures), so about 1500 stale animator copies a sweep were applied over the simulation's. The struck behaviour's lesson for the player was taken in the frame (`take_lessons`, PostUpdate) and sent the player's whole practice back, which undid the step's newer practice ticks. The `--arena-fell` debug tool wrote the opponents' health in the frame.

## What the original does

Every character is a physical body that the simulation thread ticks, and its clip clock advances in that tick. Exarust's bodies without a body tick (a level's standing characters before or without their physics) stand in for those bodies. The struck behaviour (the motile AI's, which first runs the combatant base's) runs from the damage routine's contact handling on the simulation thread. When the attacker's swing word lacks the lesson mark, it writes the struck character's combat rating into the attacker's practice lesson when it is higher, and then sets the mark. No ring post and no main-thread hop are involved.

## What Changes

- A body that neither its body tick (`TickedClips`) nor the frames (`FrameClips`) advance ticks its clip once at the end of each fixed step (`tick_standing_clips`, `FixedLast`). The tick is 0.6 × the race's base, and the sample is kept. The frames place its parts from the kept pose (`pose_standing_clips`) and leave the clip player alone. A start's pending capture is taken in the step.
- Presentation figures (the creator's, the inventory doll, the character view) are `FrameClips`. They keep the frame-time clock in menus and pauses.
- `take_lessons` runs in the step's post-step, after the hit reactions.
- `--arena-fell` posts its drop to the step (`drop_dead`).
- The bridge's write log: a frame value equal to the simulation's newer one is not counted. With `EXA_BRIDGE_WRITES`, the simulation side logs what each applied value undid, with the entity, the frame copy's base step and the simulation write's step.
- Latch audit: the frame consumed one more step-raised latch, the standing body's pending capture, and that latch now stays in the step. The stay check was the only other one. The frame's other step posts come from input, flow steps or one-shot inserts and messages.

## Impact

- `exa_anim`, `exa_game::{player, player_practice, creator_view, doll_view}`, `exa_physics::sim_bridge`, `exarust::viewer`.
- Lockstep: a standing body's clip advances one tick a step instead of the frame time's count of ticks. The practice lesson lands in the step that dealt the blow instead of after it. Traces are compared with main in the lane's evidence.
