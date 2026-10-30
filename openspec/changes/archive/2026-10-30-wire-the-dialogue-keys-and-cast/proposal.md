# Proposal

## Why

Phase 7 (Game modes and content, story act 1): the conversation engine
runs, but nothing in the running game reached it. The dialogue keys only
existed as decoded hotkeys, the reply panel had geometry but no source, and
the script cast never saw the world's characters, so scripts read stale
names, roles and health.

## What Changes

- The hotkey handler sends the dialogue keys to the engine: the Talk key
  ends a running dialogue (`LeaveDialogue`) or talks to the focused
  character (`Talk`), the digits 1–9 pick reply 0–8 (`PickReply`).
- `exa_ui::dialogue_window` syncs the engine's reply panel into the
  measured panel view: entries measured at the panel width less eight
  units, `[Other]` appended unmeasured, laid out once per change.
- `exa_game::script_cast` mirrors each live character (name, roles, red
  health over maximum, arousal, the collapsing/dead state bit) into the
  script cast each frame; relations, topic memory and role values stay in
  the cast. A role list a script changed is written back to the AI's
  `Roles` and its change flag cleared.

## Evidence

REA and `context/re/exe.asm` on `Exanima.exe` 0.9.5: the script `Health`
property divides the red (lasting) pool by the maximum, multiplies by 100
and converts with round-half-even, giving 0 unless the pool is above 0.0;
the out-of-action test is the `0x800` state bit; the hotkey routine maps
the digits and Talk key while a dialogue is open.

## Out of scope

The focus query that fills `DialogueFocus` (so the Talk key cannot start a
conversation yet), painting the reply panel and bubbles, pointer input on
the panel, the AI contact mirror, and saving relations and topic memory.
