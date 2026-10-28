## Context

Formats page: `docs/formats/RSQ.md` (motion playback; the capture and the
torso twist). The capture rules sit beside the existing torso twist
(`body_twist.rs`) and the clip clock (`exa_game::limp::clip_tick`).

## Decisions

- The half-lean works on 3×3 frames as the original does: the relative
  turn in the twist's frame is the twist's transpose times the turn; the
  forward axis is lerped 0.5, the up made square to it (dot summed x, z,
  then y), the side their cross product; normalisations exact, as in the
  existing twist.
- Which twist: the original's starts made by the behaviour, the posture
  and the locomotion run before the body tick's twist block, so they read
  last tick's twist; a follow-up started by the sampler's clock reads this
  tick's. Exarust defers every capture to the sampler, so the twist block
  keeps `twist_before` and the clock passes the right one.
- Which bones: the midline chain the twist already uses (the original's
  class bit 0x40) with bone 8 or 9; whole captures and the hold pose are
  unchanged.
- The step gate's `blocking` word was never fed; it is the trailing-foot
  request in the original's gate, so the duellist passes
  `Locomotor::plant_request`. The tone and support inputs keep their full
  defaults (feeding them would change the live opponent).
- The placement resets and the struck item's sound stay tested functions:
  nothing in the running game calls them yet without a risky spawn change.

## Open questions

- The capture's planted-foot flag: which reader consumes it (the key
  blend's flag) — answer with REA on the key blend helper.
- The struck item record's split and length fields: confirm their writer in
  the item factory with REA before wiring the sound.
