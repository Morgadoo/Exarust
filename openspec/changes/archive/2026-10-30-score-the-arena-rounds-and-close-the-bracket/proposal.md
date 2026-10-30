# Proposal

## Why

docs/PLAN.md, Phase 7 (game modes and content), arena paragraphs: round 34's
review left the match experience award without a live damage counter, the
scene draw and the roster health gauge without callers, the auto-rematch
schedule and the single reset entry unbuilt. This change carries the arena7
lane's pass.

## What Changes

- Every fighter keeps a combat score: a wound adds 3 to the striker's, a
  clash or block taken while parrying 1 to the parrier's. The match tick's
  round end hands out the experience pool by those scores to the manager's
  and roster's records and clears the credited scores.
- A tournament's next round names the watched fight's winner with the scene
  draw; its end and forfeit clear the brackets and name the first open
  event's winner.
- The manage roster's four columns (with the 16-pixel gap) and the owner-drawn
  health gauge on the manage roster and the board's roster.
- The single arena reset, run before an arena slot load.
- `--arena-auto`: the dormant auto-match switch, with the 40-tick rematch.

## Evidence

REA/Ghidra and the retail assembly: the damage routine's three notices to the
struck body's behaviour and the behaviour's base notice (3 to the striker on
a wound, 1 to a parrying body on a clash or block); the round-end routine's
award after the round count (slot as the tier, template experience, the
trainer word); the scene draw's three callers (bracket round, forfeit,
match end) and the first-undecided settle; the manage window's constructor
(columns 0x84, 0x50, 0x10, 0x84); the owner-drawn cell's pen line; the reset
and its call before the arena load reads the header; the return's 40-tick
schedule at a timer scale of 100 ms.

## Out of scope

- The GUI's gauge shading routine (a GameGUI routine): the gauge is drawn
  with the panel and client-fill layers at the exact place and size.
- The manage, records, status, arsenal and loadout windows' control tables
  and their buttons.
- The new career's full sequence (manager kit list, stash shelving, hire
  list, tournament board) as one entry.
