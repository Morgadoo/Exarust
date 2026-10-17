## Context

How the original runs a swing once an attack has started.

**Swing routine** (every tick of an attack clip, motions 5–12, from the
body tick's attack branch, after the posture decision and before the
clock advances). In order: the condition becomes attacking and the game
clock is stamped as the last attack tick; the weapon-in-hand gate (armed
weapons that slipped out of the grip lose 0.1 of rate and skip the rest;
weapons are not bodies in Exarust yet); the window region: past 0.3 of
the clip (compared in double precision), or within the first 0.25 of a
chained one; in it the swing is late past 0.7 and each striker's tracker
takes its tip velocity (capped at 50 cm a tick up to 0.8 of the clip,
after it at the slot's best, which then only shrinks; the slot moves a
quarter of the way each tick; its squared speed at most the latest and
1000); outside it the trackers clear. Then the rate: g = 1 − 0.833·W18·
(d + d²), d = |progress − 0.5| (1 − 0.833·W18 at or before the first key);
the stab ×(1.2·progress + 0.5) under 0.4, then ×1.2 (double precision);
a feral record (its level f > 0) ×((1 − f)/2 + f·S(progress) + 1/2), S =
(3 − 2x)x² (½·(1 − f) + ½ at or before the first key); rate = W15·g·
1.125·base. A ×0.64 under a byte no code writes, and a zeroing branch the
compiler left dead, change nothing. Then a chained swing past 0.25: its
state word becomes the continued bit alone and every striker's window
shuts. Window closes: the overhead's past 0.9 with weapon class 0, else
0.95, or at once past 0.85 when the first tracker's vertical speed is
above −3 cm a tick; the stab's past 0.8 unless the first tracker drives
along the aim at 2.5 cm a tick or more, then past 0.95. Window open:
past 0.3 (stab 0.4, overhead 0.5) while not yet opened this swing: with
weapon class bit 0x80 the overhead's first held item (both with claws),
the main side's and the stab's first, the combos' second — which becomes
the primary striker; otherwise every held item. Remise: with no
released, stalling, continued or no-Remise bit, past 0.8 of a main-side
attack the other side's chains, and the reverse. The stall: P −= stall
(floor 0); once the game clock is past the stall's end the stance
restarts (blend 2·W19, smooth) and the state word clears; rate ×= P.
Tiredness last: E/Emax under 0.8 (double) → 0; under 1 → (E/Emax)² in
place of the rate. Then the attack voice and the whoosh (no sound sets
yet). Off its balance (under 0.6, double precision) the body tick takes
its falls branch instead: no routine, and the rate block writes 1.25 ×
base, which an attack's clip runs (its clock advance skips the 0.6 for
motions 5–12).

**Release** (the player's behaviour without an attack request, the AI's
choice not to continue): while attacking and no recoil, cleave or earlier
release: P = 1; released and stalling; stall = max(1 − 0.05·√s, 0.1)/W17
× 0.03 (s = the first tracker's squared speed; 1 when s is none), at
least 0.01, with the mixed single/double precision of the original; the
stall ends round(1000/(80 × stall)) ms later, at most the expected swing
time; Feint's renewal round((2f + 1) × (round(100·W17) + 100)) ms later,
f = S(progress/0.7) up to 0.7 in an attack outside a chain's continuation,
else 1. Not attacking, the state word keeps only the released, recoiled
and 0x40000 bits. A main-side attack whose stall has ended returns to the
stance (2·W19, or 30 without a primary striker, smooth), condition
normal. The clip start does not write the condition, so the behaviour
tick after a stall's stance releases once more (the original does too).

**Expected swing time** (ms, at 80 ticks a second): (1 − progress) ×
T_last / (80 × W15 × ḡ × 1.125 × base) × 1000, ḡ the mean of g over what
is left: (g + g_end)/2 past 0.5, else ((0.5 − p)(g + 1) + (g_end + 1)/2)/
(1 − p)/2, g_end = 1 − 0.62475·W18; never negative.

**Daze stall** (from the daze, before damage, when the victim's condition
is attacking and P is not spent): reach = min(0.005 × the larger
tracker's squared speed, 1); r = 4·D·max(f, 0) − (1 − D)·reach·W17, D the
daze capped at 0.9, f the daze's second factor (0.25 over the most
health, ×0.5 under a potion, × the Security term when perks exist); r ≥
0.001 starts the stall (r, uncapped) or adds to it (cap 1); the stall
ends round(80·P/stall) ms later (the original's 80, not 12.5), at most
the expected swing time.

**Executor busy path** (now before the stall's end): only with the
released bit: within 300 ms of the side attack's start the overhead
(blend 0, whatever was asked); else with Feint and the renewal time
reached: a stab (blend 10, after the facing gate), a double click's
overhead (blend 10), or a side attack (the side wrappers drop the blend:
side 1 none, side 2 round((20 − W19) × 0.6)). Free path: with the
released bit the same overhead within 300 ms (before the cooldown), and
Feint's cooldown is 0 once the renewal time is reached (nothing before
it).

**Chains.** A side starter called by the routine with the current motion
on the other side keeps the primary, starts the chosen clip with
round(6·W18) + 2 frames (smooth), sets the continuation bit and restores
every striker's window after the clip start (which shuts them). A fresh
start shuts the windows, clears the trackers and sets the state word to 0,
or 0x80000 without Remise for a side attack; every start sets P = 1 and
stall 0.

**Body turn while attacking.** (1 − 0.6·enc)·(1 − 0.25·progress)·B·W16·
P / scale a tick (B = 4π/3 / 83), × |err|/0.6 under 0.6 rad, the step
divided by √scale.

The research notes, with addresses, are git-ignored:
`context/re/master-combat.md` §1.3–1.4, §2.3, §3, §4, §7.1–7.3 and its
"M22 findings" (added by this change), `context/re/joint-limits.md` §S.5,
`context/re/master-movement.md` §5.4.

## Goals / Non-Goals

**Goals:** the swing routine's rate, window, Remise, stall and tiredness
in the original's order; release and the stance return; the struck
attacker's stall; the executor's busy path and Feint; perks from the
records; the attack turn; the trace and debug helpers to compare.

**Non-Goals:** attack arm stiffness, swing sounds, the weapon-in-hand
gate, the AI's release and choice, follow-through, recoil, nudges,
parries (Out of scope in the proposal).

## Decisions

- **One component for the swing** (`SwingState`, required by `Fighter`)
  beside `Attacker` (executor state: last attack tick, primary, side
  start, picks). The routine is one system in `FixedUpdate` after the
  posture stage and before `CharacterTicks` (the attack stage of the
  tick contract); `limp.rs` only reads `SwingState::rate`, a two-line
  edit, so M0's restructuring keeps it in its sampler stage.
- **The tracker reads `Blade::tip_velocity`** (per frame, m/s → cm a
  tick): Exarust's weapons are scene children, not bodies, until M24.
- **The player's release lives in `request_attacks`** (its behaviour
  tick), as the original's; the duellist never releases (L11 brings the
  AI's choice).
- **Picks move to `Attacker`** so the routine's Remise can pick the
  chained clip with the fighter's own generator; the player's sequence is
  unchanged (same generator, same start).
- **Detection keeps its segment sweep** (C21 replaces it) and its
  once-per-target set; only the window changes. The damage reads the
  original's swing clock (clock less blend over the last key).

## Open reverse-engineering questions

| question | answer / how |
|---|---|
| Does the release stall's end use the clip rate (master-combat §1.4's wording)? | No: round(1000/(80 × stall rate)), the stall rate the release just set (asm of the release: `mulss 80, stall`; `divss 1000`). Recorded in master-combat "M22 findings". |
| Feint's blend 10 for every renewal? | Only the stab and the overhead take it; the side wrappers clear it (`xor dl, dl` before the starter). |
| Where the 0.64 flag is written | No direct writer in the disassembly (one read only); its neighbour is the limp byte. RB9 stays open; not implemented. |
| The struck stall's second factor | 0.25 over the most health (×0.5 potion), joint-limits §S.5, re-read: the daze passes D capped at 0.9 and calls the stall only for condition 2. |
| Live attack durations | Not re-read live this change (the retail game was not run); the 828 ms Remise chain of the existing trace is matched by the formula (see tasks). |

Formats page: `docs/formats/RSQ.md` (Motion playback; per-tick trace).

## Risks / Trade-offs

- A click now checks the swing at once (5 ticks with the fists), as the
  original: unattended runs that pressed one frame to land a punch must
  hold the button (`--press LMB@a-b`).
- The tracker sees one velocity per frame, so at more than one tick a
  frame it repeats; the stall rates it feeds follow the frame rate a
  little until weapons are bodies.
- Without the arm stiffness rule the swinging arm stays at the relaxed
  stiffness (0.4) through the swing: reach and contact speeds differ from
  the original until that change lands.
