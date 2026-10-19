# Design

## Context

A capture record holds a name and a code. Release searches from the end. After the removed record, if any remain, the last one is published: its code, its name, and a shape. Bit 0x8000 selects the low byte. Otherwise the shape is 0 and the default is marked. An empty list publishes code 0 and no holder, and does not mark the default. The shape table is not read.

The dismiss walk then calls a follow with argument 1. A set focus runs it. Otherwise it runs only when the mask shares bit 0. The mode word keeps bit 1 and sets bit 0. The stamp copies the clock. A ready hand list copies ten words: 0x101, 0x20101, 0x102, 0x105, 0x10106, 0x107, 0x2010b, 0, 0, 0. A focus also latches and clears the pending byte. No focus and a mask of 0 changes nothing.

## Goals / Non-Goals

**Goals:**

- Two captures, release the top. The lower code and name are published. Code 0x15 marks the default. Code 0x8007 asks for shape 7 and does not mark the default.
- The only capture publishes code 0 and no holder.
- Mask 1 and a ready hand list copy the ten words, set the mode word, and stamp the clock. The physician still leaves.
- Mask 0 and no focus leaves the hands unset.
- A focus with mask 0 still runs, latches, and clears pending.

**Non-Goals:**

- The shape table behind a non-zero shape.
- The virtual call on the focus path.
- Choosing a marked body in place of the ready hand list.

## Decisions

1. `Capture` stores the name and the code. The published cursor is separate fields.
2. The follow runs at the end of the arena begin, after both lists.
3. `TGUIForm` moves from 2% to 3%. `SG_PlayerControls` stays 8%. The UI section stays 0%. `XA_Arena` stays 34%. The game-modes section stays 0%. The runtime section stays 4%.

## Risks / Trade-offs

- Shape 7 is recorded and not drawn. The table was not read.

## Migration Plan

- Release gains the publish. Begin gains the follow. The close and the leave stay as they are.

## Open Questions

- None for the published record and the ten words.
