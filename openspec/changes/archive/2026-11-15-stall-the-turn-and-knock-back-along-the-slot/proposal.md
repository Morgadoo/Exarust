## Why

Roadmap task B4 (lane `motiledyn2`), the open TMotileDynamics rows of the
function ledger. The first motiledyn lane compared parts of the body tick
and the damage routine with the assembly; this one compared the turn, the
movement decision and its gates, the move and stop branches' stance
factor, and the damage routine's block and knockback paths. Four
differences were found:

- The turn holds while a counter the impact routine writes (round(5√I)
  after a stab's or overhead's hard weapon-body hit) is 1 or more; the
  body tick takes one off it every tick. Exarust kept the counter with no
  writer and the turn with no gate.
- The arms' turn floor (0.4·|error|, at most 0.4) reads the turn error the
  posture block finds ahead of the turn: last tick's, before that tick's
  step, 0 after a held turn. Exarust read what was left after this tick's
  step.
- The turn lowers the support adapter by 10·|error| (floored at 0). The
  adapter has no reader beyond a part field; written for the record.
- The hard impact's interrupt sets the stall rate to 1 and the swing
  strength to 0, and a stab's or overhead's hit sets the strength to 0.
  Exarust swapped the two fields (strength 1, rate 0), so an interrupted
  swing played on at full rate.
- The knockback step follows the striker's tracked slot vector (cm a
  tick, not normalised), so most blows over 0.5 take a whole step.
  Exarust normalised the tip's direction (20·J cm).

## Evidence

The disassembly of Exanima.exe 0.9.5: the body tick's turn block (the
counter's signed compare with 1 and the condition mask 0xac0, the
adapter's MAXSS against 0, the error stored before the step), the arm
setter's read of that error, the impact routine's two writes on the swing
and the counter, and the damage routine's call of the step routine with
the address of the slot vector it copied at its start (the step routine
zeroes its height, scales it by 2·amount, adds 20 × the chest's fast
velocity and cuts it to the step length).

## What Changes

- `exa_physics::body::BodyTimers`: the countdown is the turn stall
  (`stall_turn`, `turn_stalled`), and `turned` lowers the support adapter.
- `exa_game::stages::movement`: the turn holds while stalled; the bearing's
  turn is last tick's error (`limp::bearing` too).
- `exa_combat::impact::strike_limbs`: the interrupt's and the stab's
  writes on `SwingState`, and the stall on the striker's body.
- `exa_combat::knockback_offset` takes the slot vector; `react_to_hits`
  passes the striker's tracker of the slot that struck (the tip's
  velocity in cm a tick without one).

## Impact

Traces change in any fight: an interrupted swing now freezes until its
stance returns, a stab or overhead that hits a weapon body hard holds the
striker's turn, the arms' turn floor lags a tick, and heavy blows step the
struck body a whole step. Owed once the GPU is back: the determinism grid
(`arena --opponent`, `exanima01`) and the grid scenes `play arena
--opponent --press LMB@…` (a duel with clashes and knockbacks).
