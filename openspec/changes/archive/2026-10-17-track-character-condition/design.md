## Context

The original's body tick writes the condition word in its posture block,
after the daze block and the behaviour update and before the attack and the
clip clock; the swing routine, the get-up start, the clip end, the walk's
stop, the collapse start, the dying tick, death, the knock-out and the AI
write it too. The research notes, with addresses, are git-ignored:
`context/re/master-body.md` §3 (values, decision order, writers, flags),
§10.2 (the body tick's order) and the new §20 (this change's reading and
corrections), `body-feet.md` §10 (the falls block), `body-tick-order.md`
§4 (the limb step's summed limb motion), `corpses-and-markers.md` §3.6
(the cursor's masks), `master-plan.md` F1 and its K.4 (move `Health` and
the condition into their own module).

Another lane splits the character tick into stages (M0). This change keeps
the posture decision in its own module and system set, so it moves onto
M0's posture stage unchanged.

The format page this change extends is
[`docs/formats/RSQ.md`](../../../docs/formats/RSQ.md) (Physical body: a
new Condition section and the trace's column).

## Goals / Non-Goals

**Goals:**
- one condition word per character, the original's values and masks, in
  one Bevy component every system reads;
- the posture decision as a pure function in the original's order, pinned
  by unit tests to its thresholds, and a system that writes it once a tick
  before the clips advance;
- the writers Exarust's existing events stand for (get-up start, clip end,
  killing blow, placed corpse);
- every `Fighter::down` and ad-hoc down test replaced by the word.

**Non-Goals:**
- the posture block's actions (falls, staggers, Defence and Rebalance,
  get-up decision, the weak Defence, kick, switch, powers): C11 and later;
- the collapse (L5), knock-out and dormancy (C23), shocks, fall death;
- replacing `Fallen` (Exarust's fall until C11) or `Attacking` (the
  attack's data).

## Decisions

- **Where it lives.** `exa_combat::condition`, beside the new
  `exa_combat::fighter` (`Fighter`, `Health`): every consumer already
  depends on `exa_combat`, and the plan moves `Health` out of the attack
  code. `damage` re-exports `Health` and `DEATH`, so lanes that import them
  from there still build.
- **The decision returns its branch.** The original's tests that write
  the word also choose what the body tick does next. `posture` returns the
  word and the branch (collapse, dead, shocked, getting up, falling, lying
  still, attack, weak, standing), so C11 attaches the falls and the get-up
  decision and L5 the dying tick without repeating the tests. The summed
  limb motion only splits lying still from falling: the word is down under
  0.3 either way.
- **Order in Exarust's tick.** `PostureTick` runs after `StanceTick` (the
  daze's stance, as the original's daze block comes first) and before
  `CharacterTicks`; Exarust's fall start runs after it. The system first
  applies the daze's writer (normal unless helpless or attacking, read
  from last tick's word as the original's daze block does), then the
  decision from last tick's balance, summed limb motion and tone and the
  clip playing now (the player's behaviour runs in `FixedPreUpdate`, before
  it, as the original's behaviour update precedes the posture block).
- **Writers between ticks.** Exarust's get-up starts in the frame
  (`balance::get_up`); it writes getting up there. The clip end writes
  normal inside the character tick, the same tick the get-up clip ends
  into its stance, as the original's clock section does. The killing blow
  writes dead (L5 replaces it with the collapse). The walk's stop (1 → 0
  with a stance start) needs no writer: Exarust's steering starts the
  stance before the posture stage, which then sees it.
- **Summed limb motion.** The original sums, over the living limbs, each
  limb's largest particle displacement of the step before; Exarust sums
  each part's farthest particle over the last tick (positions − previous,
  as its sleep countdown reads it), one tick fresher. It only splits two
  branches that write the same word.
- **Readers.** Masks are named by what they hold: `helpless` (0xAC0),
  `fallen_or_dying` (0xA00, which holds down too: the notes' "dead or
  collapsing" is corrected), `off_feet` (0x200), `dying` (0x800), `busy`
  (0x10203). Steering and attacks stop while helpless (the original skips
  the turn and the attack engage for 0xAC0); the duellist idles for 0xA00
  and leaves a dying target; hit detection skips only the dead, as before;
  the cursor maps dead, collapsing and bit 0x200. Exarust's `Fallen`
  stays beside the word in the readers that had it, since its fall starts
  above 0.3, where the word is still normal.
- **Exarust's stand-in clips.** Its backpedal and combat walk are not the
  original's walk or run clips: they give normal (the original plays the
  walk backwards, 1, and its footwork plays the stance). They go with M16
  and C16.

## Risks / Trade-offs

- The posture system needs a living body (a ragdoll with muscles) and its
  animator; races without bodies (M48) keep the word they spawned with.
- Between the get-up's start in the frame and the next tick the word may
  lag a frame behind the clip, as the start itself does.
- `Fighter::down` and the move of `Fighter`/`Health` break other lanes'
  code that names them: `Condition::is_dead()` / `dying()` replace `down`;
  `exa_combat::{Fighter, Health}` and `damage::{Health, DEATH}` still
  resolve.

## Open Questions

- Which of the original's systems the AI's targets skip (collapsing
  bodies included?): answered with the AI's perception (L11, M11) from the
  readers of the word in its target scan.
- The state flags' bit 0x1000 at spawn: settled when inventory and the
  interface land (master-body §14), from its readers in `exe.asm`.
