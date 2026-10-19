# Design

## Context

The arena branch of in-game command 4 calls a dismiss walk before the specialist leave. The walk reads a count, and if that count is negative it does nothing. Otherwise it visits children from the last index down to the first. A child is dismissed when its flags masked by 0xce100000 equal that mask. The call is virtual and passes 0. A second list is walked the same way. A null list is skipped. A follow-up call after both lists is a separate function.

## Goals / Non-Goals

**Goals:**

- `begin_match` dismisses marked children before `leave_before_match`.
- Last-to-first order is visible. The first list finishes before the second starts.
- Flags 0xce100000 and any value that still contains those bits are dismissed. A value missing one of those bits stays.
- Count 2 ignores a marked third child. Count 0 and count -1 dismiss nobody.
- Story mode does not dismiss, and does not remove the physician.

**Non-Goals:**

- The virtual method body. A dismissed child is marked; it is not removed from the list.
- The call after the two lists.
- A count past the list. Exarust stops at the list. The image would read past it.

## Decisions

1. The two lists and their counts live on `MatchGate`, so `begin_match` keeps its arguments.
2. Dismissed names are stored in walk order.
3. `TGUIForm` stays 0%. `XA_Arena` stays 34%. The game-modes section stays 0%. The runtime section stays 4%.

## Risks / Trade-offs

- Marking a child dismissed is not the virtual method. The method was not read.

## Migration Plan

- The begin command gains the walk. Hire, the leave, and the hub flag stay as they are.

## Open Questions

- None for the mask and the reverse walk.
