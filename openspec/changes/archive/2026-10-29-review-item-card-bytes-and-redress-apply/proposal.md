## Why

Round 30's item GUI lane stalled after three commits and wrote no report. Its sentence-source change, model marks and start-up sheet loader did not match the instructions; its byte-wise name edit and the body refresh did. This change records what was kept, corrected and removed.

Phase 4 (combat and items) in docs/PLAN.md.

## What Changes

- The card's weapon sentence edits the name as bytes: a bracketed name shorter than four bytes becomes empty, and a multi-byte character can no longer split the string.
- The apparel sentence's material splice follows the in-place edit at the string's end: a mark that is the last byte takes the word and no space.
- The card keeps the weapon page's precedence over the apparel body (the lane had reversed it).
- The doll's apply keeps the stowed ids the plan read, refreshes the body when both are empty and clears the body's weapon part once, between the detaches and the attaches, when its class word has no `0x30` bits. The lane's merged model marks are removed and the take-in and shield helpers restored.
- The sprite-sheet crop stays a tested function; its start-up loader is removed because nothing draws the cells.

## Evidence

The card builder's page and sentence branches (the apparel body is looked up only when no weapon page was built), the weapon sentence's `SetLength(len - 4)` on a bracket, the apparel sentence's splice loops, and the re-dress plan, apply and take-in routines of the character view's equipment cluster, read in the instructions.

## Out of scope

The apply's early cancel and its body call between the refresh and the stance; per-kind model marks of the re-dress plan (owner word, slot); drawing the rating dots, coins, grip buttons and sockets.
