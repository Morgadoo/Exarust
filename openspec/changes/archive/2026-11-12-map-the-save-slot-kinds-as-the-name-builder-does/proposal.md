## Why

Roadmap task B0.1 (save slot kinds). The round 202 gamemain lane read that
the original's save-name builder maps slot kind 1 to `.rsg` and kind 2 to
`.rcp`, while Exarust's character-screen select and saves screen treated
kind 2 and availability bit 2 as the save. The reset to the portal and the
delete box delete files, so the mapping had to be settled first.

## Evidence

REA (Exanima.exe), checked against the assembly:

- The name builder indexes a 16-byte extension table with `kind − 1`; the
  table holds `.rsg` then `.rcp`. The empty-slot creator builds kind 1
  (the save) and the checkpoint routine tests kind 2 (`.rcp`) before it
  copies the save over it.
- The character screen's select opens kind 2 first (availability 2) and
  kind 1 next (availability | 1); it reads kind 2 when bit 2 is set and
  the asked kind is 2 or bit 1 is clear, else kind 1 when bit 1 is set.
- The full load tries kind 2 then kind 1 when asked for kind 2, else kind
  1 then kind 2; the screen's CONTINUE passes the form's kind to it.
- The form's select defaults kind 0 to 1, moves an asked 2 without bit 2
  to 1 and an asked 1 without bit 1 to 2; Previous, Next and the combo
  pick use kind 1; the toggle swaps when the other bit is set. The
  creator's show in continue mode sets the kind to 2 when the
  availability is exactly 2, else 1.
- The delete builds and deletes kind 1 then kind 2. The reset's Yes
  deletes the open package (the file the select read), then keeps the
  slot only when kind 2 (`.rcp`) still opens.

So kind 1 is the save and kind 2 the checkpoint. Exarust had them the
other way round in the select and the CONTINUE: a fresh screen read and
continued the checkpoint when there was one, and the reset to the portal
deleted the checkpoint instead of the save (dropping the slot while its
save stayed on disk).

## What Changes

- `SlotKind::code`/`from_code`: the name builder's numbers (1 save, 2
  checkpoint).
- `SlotList::character_screen_select`: availability bit 2 is the `.rcp`,
  bit 1 the `.rsg`; kind 2 reads the checkpoint.
- `SaveScreen`: the CONTINUE prefers the checkpoint only for kind 2; the
  screen opens at kind 2 when only the checkpoint is there, else kind 1;
  `--load-slot n` defaults to the save.
- Tests pin the mapping over built save folders (select, continue, reset,
  delete); docs (`SAVES.md`, `UI.md`, `DEBUGGING.md`) follow.

The character creator's own pure model (`create.rs` `SaveKind`, whose
names are swapped though its numbers are right) is left to the creator's
session.
