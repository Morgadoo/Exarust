## Why

Exarust has no shared word for what a character's body is doing. Whether a
fighter is dead is a `Fighter::down` flag set by the killing blow, whether
it lies is Exarust's `Fallen` component, whether it attacks is the
`Attacking` component; the cursor, the player's steering and attacks, the
duellist, the daze and hit detection each test their own mix. The
original keeps one value on every character, the condition word, written
once a tick by its body tick's posture block from the clip it plays, its
balance and its tone, and by a few events (a swing, a get-up, a killing
blow, death); every system tests it by a handful of masks. The collapse
before death (L5), the falls and get-ups (C11), regeneration (M43), the
damage rules (M29) and the knock-out (C23) all read and write it.

Phase 3 (Bodies), milestone M3 "standing man", and Phase 4's M4 "first
duel". This is change M38 of the master plan (`context/re/master-plan.md`,
lane F, F1; body note B1).

## What Changes

- New `exa_combat::condition`: the `Condition` component with the
  original's values (0 normal, 1 walking, 2 attacking, 8 combat idle, 0x20
  casting, 0xC0 shocked, 0x100 weak, 0x200 down, 0x300 getting up, 0x600
  dormant, 0x800 collapsing, 0xA00 dead, 0x8000 kicking, 0x10000/0x10004
  switching weapons) and its masks (0xAC0 helpless, 0xA00, 0x200, 0x800,
  0x10203); `StateFlags` (dormant, waking, held, shock stagger, potion),
  which nothing sets yet.
- The posture stage (`PostureTick`, after the daze's stance and before the
  character ticks): each living body's word once a tick, the daze's writer
  first, then the posture decision (`posture`) in the original's order,
  which also names the branch the original takes (collapse, falls, lying
  still, getting up, attack, weak, standing) for the changes that add its
  actions; `Ragdoll::exertion`, the summed limb motion it reads.
- Writers between ticks: the get-up's start (getting up), the clip end of
  a clip played once (getting up → normal), the killing blow (dead, until
  the collapse lands), placed corpses (dead).
- New `exa_combat::fighter`: `Fighter` and `Health` (moved out of `lib.rs`
  and `damage.rs`); `Fighter` requires the condition and the flags;
  `Fighter::down` is gone.
- Readers move to the word: hit detection skips the dead; the daze leaves
  a helpless or attacking fighter's clip alone; the stance part skips the
  collapsing and dead; the player is not steered and engages no attack
  while helpless; the duellist idles while down, getting up, collapsing or
  dead and leaves a collapsing or dead target; the player's target is any
  fighter not dead; the world cursor reads dead, collapsing and down (bit
  0x200) from it; the body's turn is 0 while helpless.
- `--trace` gains a last column, the condition word in hex.

Evidence:
- REA and `exe.asm` on `Exanima.exe` 0.9.5: the posture block of the body
  tick (every branch and constant: balance 0.6 and 0.3 in double, limb
  motion 25 cm, tone 0.85 against the image's float, the weapon switch's
  0.6), the swing routine's write of 2, the daze block's write of 0, the
  get-up start's 0x300, the clip end's 0x300 → 0, the stop of a walk's
  1 → 0, the collapse start and dying tick (0x800), death (0xA00), the
  masks the player behaviour, the cursor request and the AI test; the limb
  step's summed limb motion (`context/re/master-body.md` §3 and the new
  §20).
- The installed game: `humanoid.rsq`'s action indices (the clips the
  decision tells apart), pinned by the existing motion-set corpus tests.

Out of scope:
- the posture block's actions: the falls, staggers, Defence stance and
  Rebalance (C11, with the stagger's 0x300), the get-up decision's own
  rules (C11), the weak branch's Defence stance (nothing lowers tone
  yet), the kick, weapon switch and powers (none start in Exarust), the
  collision layer the original restores on leaving a state of 0x100 or
  more (C17);
- the collapse before death (L5, next), knock-out, waking and dormancy
  (C23), fall death (M44), shocks;
- the state flags' writers (knock-out, AI dormancy, shocks, potions);
- `Fallen` and `Attacking` stay (Exarust's fall and the attack's data);
  C11 replaces the fall.

## Capabilities

### New Capabilities
(none)

### Modified Capabilities
- `character-stats`: the condition word, its posture decision and its
  writers and readers (new requirements).
- `physical-body`: the body trace's condition column (modified Body
  trace).

## Impact

- `exa_combat` (`condition.rs` with `condition_test.rs`, `fighter.rs`,
  `lib.rs`, `damage.rs`, `stance.rs`)
- `exa_physics` (`Ragdoll::exertion`, a small edit in lane A's `body.rs`)
- `exa_game` (`limp.rs`: the clip end's writer and the turn; `balance.rs`:
  the get-up's writer, the fall after the posture stage; `cursor.rs`,
  `player.rs`, `trace.rs`, `hud.rs`), `exa_ai` (the duellist), `exarust`
  (`viewer.rs`: placed corpses dead)
- `docs/formats/RSQ.md`, `docs/PLAN.md`, `docs/PLAYER.md`,
  `docs/DEBUGGING.md`, crate READMEs
