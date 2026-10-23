# Proposal

## Why

PR #2 (94 commits from a Cursor agent) edited `openspec/specs/*` directly,
without a change folder, and left `openspec validate --all --strict`
failing on three requirements over 500 characters (physical-body's Support
and Leg bend, motion-playback's Clip clocks). The review of PR #2 checked
its rules against the original's instructions and fixed the code on the
review branch (the cast, combat, body and test lanes, then corrections).
Several requirements the PR wrote now state rules the fixed code does not
follow, and one (bleeding over time) describes an invented duplicate of
the burning routine that was removed. This change brings the specs in line
with the fixed code and records the PR's spec edits through the workflow.

## What Changes

- combat: remove "Bleeding damages over time"; the shock stagger sets the
  Defence stance; the daze skips a parrying fighter; readiness rises
  through a parry.
- motile-part: the limb impact's gates (a third, halved down, reaching
  0.5), the overhead striker's hard impact sounding before it is dropped,
  the clock's random spread drawn after the damage step (new: Limb impact
  gates).
- motion-playback: Clip clocks (over 500 characters, its precision
  scenario wrong) is removed and split into Clip clock gains, Clock
  precision and Clip end and loop; Thorax hunch is replaced by Feral
  thorax hunch (the character's feral level gates it; only the balanced
  attack branch eases it); the clip rate keeps its last value when weak
  or walking, and the stance reset reads last tick's parry and runs while
  shocked (new: Stance rate reset); the lying foot pitch.
- physical-body: Support, Leg bend and Velocity retention are split (new:
  Gait probe sphere and sway, Lying leg release, Living retention); the
  arm branches in the original's order with the drive zeroed every tick
  (new: Arm relaxed blend, floor and cap); the parry blend loses to a
  daze; the standing lift leads; the target cap's 5 cm added after the
  cap; the support pins applied in sequence (new: Support pin power); the
  stand-still bands end at 1.5 (new: Stand-still stamps); the pair
  impact's zero cosine (new: Hit event pairs); new Dormant sleep countdown
  and Death clears the feet's strikes.
- npc-steering: new Entering resets the body and A hidden character.
- player-movement: a walk ends into idle with the fast ease; the footwork
  step is measured from the chest (new: Long step latch).
- thaumaturgy: the kinesis reach from the held part's child parts and the
  waiting kinesis's reach answer; a throw refused by an empty kinesis; the
  caster's technique word scoped to duellists and placed characters; the
  blast, bubble and wave ends; the hook answers with the vanish exception;
  the player's hook asks (tone gate, sprinted move). New: Blast shortfall
  and throw, Wave throw, The cast end, A replaced motion asks the hook,
  Vanish let-go, A later cast is a new hook, The release flag asks the
  hook, The player's caster, A hidden duellist casts nothing.
- `docs/PLAN.md`: the PR's stale or wrong paragraphs corrected and one
  Done paragraph for the review.

## Capabilities

### Modified Capabilities

- `combat`, `motile-part`, `motion-playback`, `npc-steering`,
  `physical-body`, `player-movement`, `thaumaturgy`.

## Impact

- `openspec/specs/*` (the seven capabilities above) and `docs/PLAN.md`.
  No code changes: the fixes are already on the branch.
- Evidence: the four review lanes' readings of `Exanima.exe` through REA
  (the body tick's arm state machine, posture block and clock block, the
  death routine, the feet and footwork routines, the support-pin setter,
  the limb part's impact routine and slot routine, the burning routine,
  the item-use routine, the cast end and sustained spend, the kinesis
  tick and the hand's reach request, the vanish hook, the player
  behaviour's key-code block and hook asks, the AI manager's dormant
  branch), recorded in the review notes under `context/re/packs/pr2/`,
  and the tests the lanes added.

## Out of scope

- Code changes of any kind (the fixes are reviewed separately).
- `docs/COMPONENT-MAP.md`, `docs/reviews/*`, `docs/img/*`.
- Rules the review left open: the shock reaction's caller, the shield body
  a block would stamp, the world clock's saved base at load, the player's
  game-view and status gates around the hook asks, the slow-turn landing
  extension.

## Phase

Phase 3 (bodies: physics, control), Phase 4 (combat), Phase 6 (AI) and
Phase 7 (thaumaturgy): the review of PR #2's work in those phases.
