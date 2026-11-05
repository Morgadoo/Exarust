# Proposal

## Why

The wiring backlog listed S11, "scripted game modes and a versus lobby",
as a system to build. Its premise was that the original has playable
modes beyond story, arena and practice whose end handlers Exarust lacked.
A trace of Exanima 0.9.5 shows otherwise:

- The original keeps a table of scripted game modes. Each entry holds a
  mode word and five hooks: enter, session begin, session leave, creator
  cancel and end menu, plus the player record it kept when the mode was
  left. The mode switch, the session start, the session leave, the
  creator's cancel and the end menu all look a mode up in it. Nothing in
  0.9.5 fills the table. Its count is only read, and nothing in the image
  holds the table's address. Every scripted-mode branch is dead in the
  retail game.
- The versus lobby (mode `0x1010`) exists only as gated code. The arena
  page shows VERSUS only when a flag is set, and nothing writes that flag.
  The lobby form and the versus equipment form come from two hooks, and
  nothing writes those either. Forced on, VERSUS switches the mode word
  to `0x1010` and opens the free creator with READY. With no lobby form,
  READY does nothing, and the creator's cancel returns to the title.
- No other game mode exists: the title, practice, story and arena are the
  only words the menus set.

Exarust already modelled the table's end-menu slot and named the hooks.
Its other lookups were wrong in four ways:

- the enter hook ran for any unregistered word;
- the leave and the class-select cancel ran an "end" hook;
- the creator's cancel skipped the transition and the cancel slot;
- the class select's cancel ran a scripted hook directly, with no
  transition.

Phase: Phase 6 (story mode: session flow) and Phase 7 (arena: the
arena page) of docs/PLAN.md. This change does not edit PLAN.md, which is
the owner's.

Evidence (REA, Ghidra on `Exanima.exe` 0.9.5, checked against the
assembly):

- the mode switch, the session start, the session leave, back to the
  title, the creator's cancel and the class select's cancel;
- the end menu's click, the main menu's click and its page builder;
- the creator's final button;
- the unit's finaliser;
- references to the table, its count, the versus flag and the two lobby
  hooks. Every access was read for read or write. A byte search of the
  image found no stored copy of those addresses.

## What Changes

- `exa_game::flow::ScriptModes` is the table: registered words with
  their cancel slot, first-match lookups, held by the flow.
- `ScriptHook` names the original's slots: Enter, Begin, Leave and
  Cancel. The end menu's slot is the existing `ScriptedEnd`.
- Enter runs only for a registered word. Begin runs at the session
  begin, before the frame task. Leave runs only for a word with the
  scripted bit that is also registered. The arena's own leave keeps its
  name under the arena word.
- The creator's cancel (outside the recruit) runs behind the transition
  with the music left playing. It runs a registered mode's cancel slot,
  looked up by word alone, then goes back to the title, whose session
  leave runs the leave hook. The class select's cancel runs behind the
  transition and runs no scripted hook.
- The end menu reads the registered words from the flow's table. The
  separate `EndHandlers` resource is gone.
- VERSUS switches the mode word to `0x1010` before the creator opens.
  READY with no lobby form leaves the creator up and logs it.
- `--versus` (debug) sets the versus flag that 0.9.5 never sets.
  `--press` with a letter or digit also types it, so a scripted run can
  name a character.

## Out of scope

- A lobby form, networking or a versus match. 0.9.5 has none.
- Registering any scripted mode. Nothing in 0.9.5 registers one.
- The record each table entry keeps for its mode across switches. No
  entry exists to keep one.
- The creator and class-select forms are released when the cancel is
  asked for, not at black.
