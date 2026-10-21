# Design

`hotkey(key, bindings, context)` is the key handler's decision as a pure
function of the pressed key's virtual-key code, the binding table and what
the handler reads of the game (dead, dialogue, power bar, the permission
mask, the modes hiding the inventory or the journal, the powers). It
compares the key's low byte with each slot's word in the handler's order,
so a key bound to two slots does what the earlier test says.

The frame's system turns Bevy's just-pressed keys into virtual-key codes
(`bindings::virtual_key`: letters, digits, Tab, Space, Escape, the arrows
and the modifiers, either side) and carries out the wired hotkeys. It
replaces the two systems that read Tab and R directly, in the same place
of the frame (after the cursor ground, before the combat mode's frame).

## Open questions and how they were answered

- Which action each of slots 18–20 is: the handler opens the power bar on
  slot 18, the power tree on slot 19 and uses the picked power on slot 20
  (REA, the handler's calls), so the slots are named that way.
- The handler's flag bits 1 and 2 (likely a repeat and a modifier): Bevy's
  just-pressed keys never repeat; the second bit is not read [?].
- The player's flag that forces combat mode off was not traced [?].

No format page changes; the page extended is `docs/PLAYER.md`.
