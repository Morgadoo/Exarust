## Context

How the original moves a character, from its behaviour to its body tick
(`context/re/master-movement.md` §3–§5, re-read and extended in §21).

**Request.** Every character's behaviour writes, before its body's
movement decision in the same tick: the heading, the head's look, the
smoothed cursor direction, the wanted direction, a flag word (1 move,
2 backwards, 4 run, 8 sprint latch, 0x10 brake, 0x20 stopping, 0x40
heading-driven, 0x200 turn-about), moving, free walk and a goal point.

**Player.** Per frame, W or the right button over the view asks to move;
S too, backwards (above 1 cm a tick brake and turn-about instead); keys set
0x40, the mouse clears it; released while moving, the request holds
(0x40) until the playing clip's key segment is 0 or 4. Per behaviour tick:
Shift sets the latch, else clears it. The cursor's point on the plane 80 cm
over the ground gives a direction once 50 cm away, followed by a
second-order yaw follower (e = clamp(target − y, ±π/2), v = 0.7·(v +
0.015708·e), y += v above 1e-5). Exploration asks for the slow turn; with
no request, standing: A or D turn the heading ±90° when more than 1000 ms
(game clock) have passed since the last turn key and no object is held;
the smoothed cursor direction is mixed 0.4 with the facing and the look
eases 5 % toward the heading moved 40 % toward it; moving in free walk:
the soft stop (free walk and run off, stopping on, goal within 8v + 20 cm
ahead); moving to a goal: run off. With a request: cursor mode 3 (RMB
beyond 3 m) clears the slow turn; the wanted direction is whichever of the
facing and the smoothed cursor is nearer the raw cursor; by the mouse
beyond 120° the heading turns and nothing moves; by the mouse the heading
is the wanted direction and the look the heading; by the keys the heading
stays (A/D ±0.025 rad, stamped) and the look is 30 % toward the cursor;
free walk on, stopping off; run: cursor mode 3 sets run and keeps the
latch, else the latch is spent on run, else run off; moving on; the
look-ahead 6v² + 75 cm ahead (−70 with 0x202) walks the grid line, ends the
free walk when blocked, and within 100 cm clears run and latch and keeps
the brake; otherwise (and without 0x200) the brake is cleared.

**Decision** (body tick, muscles on): move vector = facing × 200 cm in
free walk, goal − standing while moving, else the facing; the amount =
its length². Moving: backwards → leg mode 2, brake within the stop distance
(clearing 0x200); braking → walk; run → run, brake cleared; else walk,
brake within the stop distance (never in free walk). Stop distance =
column 2 blended by the run (50 walk, 200 jog, 250 sprint, 50 backpedal
cm). Gates: cooldown > 0; E < Emax; stepping (refuses, later tests still
run); moving balance < 0.9 → cooldown 40 and landing 5; standing airborne
> 80 or balance < 0.99; Rebalance > 20 — on refusal the amount, flags and speed
are cleared. Braking below 1 cm a tick: turn-about → backpedal, else the amount = 0.
Moving and (amount > 625 or speed > 0.5) → move branch (amount = 1), else stop
branch (moving off, walk columns).

**Turn** (skipped helpless): free walk (1 − 0.1·speed)·B, slow turn 0.5·B,
attacking (1 − 0.25p)·B·handling·P·(1 − 0.6enc)/scale, else (1 − 0.6enc)·
1.25B/√scale; ×0.6 crouched > 0.5; below 0.6 rad ×|err|/0.6; moving with
0x40 ×0.9; step = rate/√scale; snap below 1e-4 or within a step; B =
4π/3/83. **Alignment**: standing 1 − enc; moving by the keys 0.4 +
0.6·(heading·pelvis)²; by the mouse 0.4 + 0.6·max(0, wanted·facing); alignment =
0.97 alignment + 0.03 target; amount ×= alignment.

## Goals / Non-Goals

**Goals:** one request every steered character writes; the original's
decision, turn and alignment each tick in the movement stage; the player's
exploration behaviour as the original's; the duellist through the request
with today's approach; a trace to measure it.

**Non-Goals:** the drive through the carry (C18), the grid line walk (M5),
combat facing and RMB walk (M2), NPC paths (M7), the head's sampler (M8).

## Decisions

- **Where.** The request and the body's state are components on the body
  model root, in `exa_physics::body::locomotion` (both `exa_game` and
  `exa_ai` write them; the rules are body-tick rules, pure functions with
  unit tests). The decision runs in `CharacterStage::Movement` before the
  gait floor and balance (`exa_game::stages::movement::decide_movement`).
  The player's behaviour stays in `FixedPreUpdate` (the behaviour stage,
  stage-major as M0 left it): it reads last tick's decision as the
  original's behaviour reads its body before the decision.
- **The capsule executes** (Exarust's own, until C18): the decision sets
  the capsule's velocity for the next step (one tick later than the old
  direct write): along the heading at 1.4 m/s blended to 3.6 m/s by the run
  blend, 1.22 m/s backwards, the interim footwork's velocity while it
  steps; braking it keeps (1 − 0.2·550/83²)·0.98977 of its speed a tick (the
  original's brake drive and limb retention) and, toward a goal ahead, at
  most v − v²/2d so it stops on the goal (the capsule has no feet to plant;
  a run braking 100 cm before a wall would otherwise hit it). The speed
  along the heading is the capsule's commanded one: its measured step
  stalls for a tick on the level mesh's seams, which stopped braking bodies
  short in game.
- **The run blend** (the part the capsule reads): running +0.05 to the
  encumbrance's cap (lower with the latch), braking or stopping at most
  f(2 − f), f = 0.2(v − 2), else −0.015, only while moving; the stop
  distance is blended from it. The alignment and slope terms are C18's.
- **Look-ahead line walk** until M5: a ray from hip height along the
  facing, clamped a capsule radius and a 30 cm cell short of what it meets
  (the grid's clamp stops at the last free cell); low edges under hip
  height are missed as before.
- **Balance gates read steady** (deviation): Exarust's standing balance can
  settle at about 0.89 after a turn with no recovery step to restore it, so
  the original's 0.99 gate held the player for good in game; the energy,
  cooldown and stepping gates are live, the balance and airborne inputs
  wait for C15 (`Footing::balanced_by`).
- **Duellist mapping.** Heading = toward its target, look and cursor alike;
  approaching beyond 1.15 m: walk flag, goal 1.5 m toward the target (the
  original AI driver's carrot), out of free walk, no slow turn; else not
  moving (it stops at once as before). Its capsule walks at 1.2 m/s
  (`CapsuleGaits`). It plays the interim walk while its body moves, its
  stance while a gate holds it.
- **Combat mode** until M2: no slow turn (300°/s standing), a moving body
  stops softly; the heading still follows the cursor beyond 30° (L1/L3).

## Open questions and how they were answered

| question | answer | how |
|---|---|---|
| RB23: W + Shift never sprints while RMB far + Shift does? | yes: the steering spends the latch on run with the keys, keeps it with cursor mode 3; locomotion reads it for the sprint row | REA on the player behaviour tick, the exploration steering and the locomotion |
| RB24: what blocks the standing quarter turn? | an object interaction held by the left button (set by the press handler, cleared on release) | REA on the field's two writers and its one reader |
| key segment 0/4 of the walk | segment 0 = before the first key and past the last (frames 0–10), 4 = (40, 50] | REA sampler; corpus (keys 10…80) |
| quarter turn spacing | strictly more than 1000 ms since any turn key, A/D steering included | REA |
| standing look and cursor mix | the smoothed cursor mixed per tick, not cumulative; the look eases 0.05 | REA (corrects §4.2) |
| retail checks RR2/RR3/RR4 | not taken: the retail game was not running | — |

Formats page: `docs/formats/RSQ.md` (Physical body: Movement decision,
the per-tick trace).

## Risks / Trade-offs

- [The capsule is a stand-in] → C18 replaces it; the decision does not
  depend on it beyond the speed it reads.
- [The hip ray misses low edges] → M5's grid line walk.
- [Balance gates off] → until C15; the trace shows the balance.
- [One tick later capsule response] → 12 ms; the clip starts the same tick
  as the capsule moves.

## Port onto main (2026-10-20)

Finished on 2026-10-17 and left unmerged; ported onto a main that had
meanwhile gained the bound controls, Cursor Look, the crouch, the sprint
latch's tighter run cap and the sprint thrust, the run overlay with the
walk played backwards, the grid look-ahead and an NPC path driver
(`exa_ai::order`, `exa_ai::path`). What changed against the change as
written:

- **The player only.** The duellist keeps main's driver (it follows its
  order's heading and velocity onto its capsule and stands on the
  position score's plateau); routing NPCs through the request is M7's.
  The movement decision runs for bodies that carry a request, so an NPC's
  capsule is left to its driver.
- **Bindings.** The behaviour reads Forward, Back, Left, Right and Run from
  the binding table (either Shift by default), Tab is Toggle Combat, Alt
  the Alternate binding.
- **Cursor Look** turns the body's yaw, the request's heading and the
  combat aim.
- **The crouch** is set by the steering before the helpless are left
  alone (moving by the last movement decision), feeds the body turn's
  ×0.6 beyond 0.5 and the bearing.
- **The sprint.** The latch is the request's flag after the behaviour
  (spent with the keys, kept by a far cursor), as main's latch was; the
  run cap reads it, the sampler's sprint test reads it, and while
  sprinting the capsule's run end takes main's feral sprint row
  (472.2/350 at feral 0).
- **The gaits.** Forward and back play the walk (main's rule: the run is
  an overlay, backing runs the walk's clock backwards); backing is the
  backpedal leg mode.
- **The look-ahead** walks the navigation grid's line walk (main's
  `exa_world::nav::line_walk`) when a grid is built, the hip ray without
  one; blocked, the free walk ends and the body brakes to the clamped
  goal, and within 100 cm the run and the latch are cut (main had stopped
  the capsule within 100 cm instead).
- **Names.** `Balance::land` was taken by main (a foot reaching its
  anchor): the landing momentum's full weight is `land_fully`.
