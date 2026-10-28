# Proposal

## Why

docs/PLAN.md, Phase 7 (game modes and content), the dialogue paragraphs
(`run-the-role-scripts`, `share-the-roles-and-script-events`): drawing the
bubbles and the reply panel, the hearing stimulus, the glow task and the
crystal walks were open; the topic memory was indexed one slot off.

## What Changes

- Topic memory buffers are the count word and one word per node, node `n`
  at slot `n` (the gate, start, offer, event, mark-used and activation all
  index by the 1-based node number); a differing stored count loads shifted
  from the stored count word into slot 1.
- Speech bubbles: width fit at 320, height `lines × line height − 2`,
  placement (narrator centred 75 % down, character head lifted 120 cm,
  rigid inverse view, viewport projection with its behind-camera edges),
  four-unit margins; the frame draws every rounded box (radius 3, alpha
  128 × fade, narrator grown by four units) before the white texts (alpha
  160 × fade) (`exa_script::bubbles`, `exa_ui::speech`).
- The reply panel's entry measure (panel width less eight units) and its
  height layout (30/17-unit minimum, narrator immediate, 500 ms animation)
  (`exa_ui::dialogue_choices`).
- A spoken line's hearing stimulus reaches both sides' contacts
  (`cast::Contact::hear`).
- `ContextObj` names a weapon "a " plus its lowered database name.
- The glow task list (schedule on use, per-frame mesh/light writes, cancel)
  and the crystal drain's socket, stack, container, record, inventory and
  collect walks (`exa_script::objects`).
- Review fixes: calm class 0 stays refused by the start/reply filter; the
  six-line candidate cap stays; the hearing compares are ordered; the glow
  writes 0 on its last frame; the behind-camera NaN edges.

## Evidence

The assembly of the conversation unit (bubble create/place/frame, hearing,
panel push/layout, topic memory load/store/open, start predicates, the
candidate records), the viewport projection and point transform, the game
unit's glow task and crystal walks, and `ContextObj` with its name lookup
and lowercase helper (REA and the objdump listing of `Exanima.exe`).

## Out of scope

Painting the bubbles and the panel in the running app; the AI driving
greetings, events and the hearing decay; applying glow writes to meshes and
lights; the object database notifications after a drain; saving topic
memory with the character.
