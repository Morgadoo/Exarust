# Proposal

## Why

Backlog item C3 asked to run the standard AI sense (`combatant::standard_sense`,
ported in round 42 and tested only) for the non-golem brains. Tracing the
original in REA shows that premise is wrong: the attach installs the standard
sense only for an AI race of class `0x12xx` (the Golem Suit) and the ally sense
for the other AI races; a race without the AI bit (every human and beast)
installs no sense, and the contact classifier then calls the observed
character's own perception, the base one every character class shares, which
is Exarust's `combatant::perceive`. Exarust already ran `perceive` for the
humans, but it also ran it for the Golem Suit, so the standard sense had no
caller.

The same trace shows two ordering differences in the AI tick: the original
classifies each contact inside the target pass, before the integration drifts
its hostility and concern (Exarust drifted first), and a brain whose body is
out of the fight (condition bit `0x800`) leaves its tick before the target
pass, so it senses nothing (Exarust's perception pass still ran for it).

Phase 6 (AI), TMotileAI.

Evidence: REA (Ghidra) on Exanima.exe: the brain's attach method, the contact
classifier (its sense call and the fallback virtual), the class tables of the
character behaviours (the fallback slot points to the base perception in every
one), the standard sense routine's listing (the statue byte it reads), the
contact integration (the target pass call, then the per-contact drift) and the
AI tick's out-of-fight exit.

## What Changes

- `exa_ai::personality::Sense` gains `Base` (no sense installed: the base
  perception), now the default; `Standard` is the Golem Suit's sense.
  `ALLY_CLASS` is renamed `STANDARD_CLASS` (it was the standard class).
- `exa_ai::brain::perceive_all` runs `perceive` for `Base`,
  `standard_sense` for `Standard` and `ally_sense` for `Ally`; it skips an
  observer whose condition has the out bit; it logs at info each contact a
  sense starts or stops tracking and each third party the standard or ally
  sense picks, and `Npc::of_record` logs the sense it installs.
- `combatant::standard_sense` takes the observer's statue byte, which its
  lying branch reads (not the observer's sleep).
- `exarust --opponent-race <n>` (debug): `--opponent-brain` with the brain
  made from race byte n while the body stays human, so a live duellist can
  run the Golem Suit's standard sense (18) or the ally sense (16).
- `exa_ai::contacts::integrate` splits into `aim` (the aim point, freshness,
  the soothe damping's step) and `drift` (the per-contact drift and the
  aggregates); `Npc::run_pass` runs `aim`, the target pass, then `drift`, after
  the pass's level, attention and stress are stored.

## Out of scope

- The original runs the target pass (sense and classify) for one brain a tick,
  the one that has waited longest; Exarust still runs it for every brain each
  tick.
- The integration's alternative hostility rule for a body whose second flag
  word carries `0x80000` (`Integration::marked_body`) still has no setter.
- The Golem Suit in the shipped levels (exanima08) lies dead, so no shipped
  level runs the standard sense live.

## Impact

`exa_ai` (personality, brain, combatant, contacts) and its README;
`exarust` (the debug flag), its README and docs/DEBUGGING.md.
