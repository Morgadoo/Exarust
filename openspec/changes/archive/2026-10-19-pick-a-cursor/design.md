# Design

## Context

After the gate hook sets its flags to 1, a set mid byte still returns. A clear mid byte picks only when the world is open, nothing is busy, the alternate hook is clear, the hold byte is clear, and a set bank side has a world aim. The raycast is a result the caller supplies. A taken item publishes a follow hook. Kind 0xa00 counts a chest. Two list-id compares are collapsed to a named row and a listed row, so no retail ids enter the tree. That collapse is an Exarust choice.

The click hook copies an aim whose bits are above 0x428c0000, the bits of 70. A miss with the hold byte clears the aimed word. A taken item sets flags 0x11. The rest of that body is not in.

The hover hook publishes a follow hook, clears a byte and a mark, and stores flags 0x15.

The focus method that runs before the hand words is still not a fixed function. A focus begin only counts the call.

## Goals / Non-Goals

**Goals:**

- A taken item on an open world ORs the gate flags with 5.
- A set hold byte does not pick.
- A fast aim copies its place, and a miss clears the aimed word.
- A hover stores flags 0x15.

**Non-Goals:**

- The pick raycast.
- The rest of the click path.
- The focus method body.
- Loading the Steam library.

## Decisions

1. The aim speed is stored as the bits of a positive finite float. A greater bit pattern is a greater speed. Equal bits do not copy.
2. `TGUIForm` stays 4%. `SG_PlayerControls` stays 8%. The UI section stays 0%. `XA_Arena` stays 34%. The game-modes section stays 0%. The runtime section stays 4%. The published headline stays 14.4% and 5.7%.

## Risks / Trade-offs

- The raycast and the two list-id compares are results, not their callees.

## Migration Plan

- `run_gate` grows the pick. `run_click` and `run_hover` are the two new bodies. A focus begin still only counts.

## Open Questions

- The focus method body has no fixed procedure. It stays a count.
