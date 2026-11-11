## Why

Phase 7 (Thaumaturgy) in docs/PLAN.md, SG_Thaum_Mind (unit routines), the
mind2 lane (round 179). After round 178 three frame-side mind writes were
left (Control Mind's handover and hand-back, Spirit Form's end), the class
of write that made Spirit Form's end intermittent in free mode. The Mind
casts' AI changes (`MindAi`) never reached the live brains, so Calm,
Enrage, Confusion, Fear, Embolden, Trust, Loyalty and Command Dead did not
change what an NPC does. The revival's gait scale, pace and stamina were
logged but not written, and the dead powers' character flags were always 0.

## Evidence

REA (Ghidra and the assembly of `Exanima.exe` 0.9.5):

- Calm, Enrage, Confusion, Fear and Embolden write the target's AI words
  the AI tick reads: Calm settles each contact's concern and hostility into
  its noticed level (its loop runs to the count inclusive, one entry past
  the live contacts) and lowers the panic, the arousal and the alarm, then
  writes the calm (the soothe damping) 1; Enrage and Confusion add 50 panic;
  Confusion clears the body's target, the brain's level and seven contact
  words; Fear adds 20 alarm (40 at most), Embolden 50 courage (80 at most).
- The morale tick decays the stress by 0.01 and the alarm by 0.9999; the
  target pass classifies with stress plus alarm less courage and raises the
  alarm to twice the alarm mean; the flee start adds 2 to the alarm.
- Loyalty and Command Dead settle the contact toward the caster (concern
  and hostility 0, noticed 30) and set the AI's re-arm byte, which the AI
  tick answers by applying the roles again (its apply reads the body's
  powers: a binding marker makes it follow).
- The revival writes the body's pins factor `1.25 − f²/4`, the root's link
  iterations `round((1 − f/4)·1660/83)`, the most energy, the step reach
  `(1 − 0.3f)(1 − 0.2·enc)·80·√scale` and the body's feral copy.
- The dead powers refuse a record whose profession word has bit 0x40 and
  revive only when its race byte (0xFF00) is clear.

## What Changes

- `exa_game::mind::runtime`: `possess`, `release` and `spirit_ends` post
  their status words and the possession marker to the world that walks
  the powers; the way home gives the caster's body the body's status word
  there. `cast_mind` returns the cast's AI changes and the bar posts them
  to the step's world (`post_mind_ai`, `drive_mind_ai`), which runs them on
  the brain's own values. The markers' end re-arms the brain.
  `MindLive` reads the record's profession word and the body's
  encumbrance and build; `raise_corpse` writes the revival on the body.
- `exa_game::mind::casts`: `MindAi` names the brain's words and records
  each change (`MindAiChange`), applied by one routine.
- `exa_ai`: `Npc::reapply` (the re-arm byte) and `Npc::apply_with` read the
  body's powers (`MindPowers` gains the resist bit); the target pass reads
  and raises the alarm instead of the stress.

## Impact

- Mind powers move the live brains (grid logs: `mind ai:` lines); a bound
  or commanded body follows when re-armed.
- The target pass's stress change alters NPC flight in exanima01; the
  arena duel (a duellist, no brain) keeps its trace.
