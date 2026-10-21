# Design

The hand keeps the left press (pointer, logical and window pixels, real
time) until its release. The press system watches it each frame while the
button is held and starts the drag once the pointer passes the allowance,
running the existing press rules with a ray through the pressed point. The
hold system treats a release with no drag as the click.

The allowance is computed in integers as the interface does (window pixels,
real milliseconds, i64 division), including its zero allowance between 181
and 190 ms.

## Open questions and how they were answered

- Which events the cursor actions 0x4001, 0x2001, 0x8001 and 2 are: traced
  in the interface's mouse handlers (REA): drag start, click, double click
  and right press; the earlier notes' "left press" for 0x4001 was a guess
  and is corrected.
- The drag allowance's constant: 2, written by the desk constructor (REA).
- A scripted pointer never moves, so unattended runs click rather than drag;
  this follows the original, whose drag needs pointer motion.

No format page changes; the page extended is `docs/PLAYER.md`.
