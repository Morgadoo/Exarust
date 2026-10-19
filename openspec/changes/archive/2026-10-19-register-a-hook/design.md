# Design

## Context

The primary list starts one below zero. A register stores the next index. A code whose high nibble is clear gains bit 0x1000. A drop removes every row with that hook and closes the gap. The ordered list inserts before the first key that is greater, and a drop removes the first row whose hook and owner both match.

A shown value is the action set. The two platform words are the interface and the controller. The resolved target activates that set. The library that provides it is not loaded.

## Goals / Non-Goals

**Goals:**

- Hold 0 registers code 0x8001. Hold 1 registers nothing.
- World byte 5 registers the wide hook at 0x2001 and at 0x1002, then the gate hook.
- Raising the mark leaves the primary list empty and inserts the watch hook at key 4.
- Value 42 activates the action set once.

**Non-Goals:**

- Loading the Steam library.
- The bodies of the registered hooks.

## Decisions

1. Hook identities are fields the test supplies.
2. An empty primary list uses a high of -1, so the first row is index 0.
3. `TGUIForm` stays 4%. `SG_PlayerControls` stays 8%. The UI section stays 0%. `XA_Arena` stays 34%. The game-modes section stays 0%. The runtime section stays 4%. The published headline stays 14.4% and 5.7%.

## Risks / Trade-offs

- Recording the activation is not a call into the Steam library. That library is outside this runtime.

## Migration Plan

- A cleared mark now registers. A raised mark drops. A show marks the action set.

## Open Questions

- None for the list index, the code bit, and the action-set arguments.
