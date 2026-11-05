# Proposal

## Why

Round 53's mind-effect runtime (`exa_game::mind::runtime`) left the other
Mind classes unticked: Sense Minds, Mask Mind, Confusion, Enrage, Trust,
Suspicion, Loyalty and Command Dead cast, but nothing expired them, so the
confused, enraged and masked bits stayed on and the markers never ended.
Spirit Form's screen, music and the Mind sounds were only logged, and the
mind pick took every body as seen and none as warded.

Phase: the Phase 7 thaumaturgy milestone of docs/PLAN.md (no PLAN.md edit
in this change).

Evidence (REA, Ghidra on `Exanima.exe`, the assembly read for the timed
classes' compares): the control handlers of the instant and persistent
Sense Minds and Mask Mind, Confusion, Enrage, Trust's and Suspicion's
markers, Loyalty's caster and marker (and their unlinks), Command Dead's
caster and marker (and theirs); Loyalty's and Trust's casts; the spirit
screen's build, start, frame hook and stop, Spirit Form's cast, control,
end and way home; the overlay music's fade-in and fade-out; the mind pick
and the level's light-reach callback it asks; the armour pass's mind-ward
writes; the contact integration's enraged-body branch; the spirit body's
builder.

## What Changes

- `mind::runtime::tick_minds` ticks every Mind class with the original's
  durations and clocks, clears the status bits, ends the markers' other
  halves, re-arms the AI and plays Loyalty's end sound.
- The casts build Loyalty's and Command Dead's casters (and the
  persistent Sense Minds and Mask Mind) running, as the original's do
  (their ready state is a save's relink, which would link a second
  marker), and Trust's marker carries its caster where the perception's
  charm test reads it.
- The enraged bit feeds the contact integration's marked body.
- The mind pick skips a body wearing a mind ward (the rules bit 0x10) and
  asks the level's light-reach query instead of taking every body as seen;
  Raise Dead and Command Dead pick with corpses allowed.
- The casts' sounds play; Spirit Form fades `Msc_SprtRealm` in over the
  level's track and out at its end, plays the screen's and the end's
  sounds, and its screen drives the post-processing's desaturation.
- `exa_audio`: a `MusicOverlay` request for the overlay track.

## Out of scope

- A physical wraith: the original builds the spirit through the one
  character placement (twelve upper parts, no legs, hovering feet), but
  Exarust's balance, lift and pins need two legs; the spirit still stands
  where it is built.
- The spirit screen's two scene-environment values and its noise texture
  (not identified; the final pass never samples the texture).
- The Sense Minds overlay's draw, Command Dead's laying down of its body
  (the running game raises no corpse), the AI values the casts move
  (`MindAi`) reaching the live brains.
