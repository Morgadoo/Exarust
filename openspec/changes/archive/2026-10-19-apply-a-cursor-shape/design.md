# Design

## Context

The shape table is filled at run time. Each record is an image and an optional hook. Shape 0 uses a separate default image. A new image is shown only when it differs from the current one and is not zero. The same image returns at once. Shape 0 then marks the default and stamps the clock. Shape 1 and up read the record at that index minus one, show its image, and run its hook when one is set. A shape past the table is not read.

A focus is invoked before the hand words and the latch. The method body was not identified: the object is chosen at run time. After the latch, a session mark that is not already 0 is stored as 0. The rest of that leave path is not in.

## Goals / Non-Goals

**Goals:**

- Shape 7 with image 42 and a hook shows 42 once and runs the hook. The same image does not show again, and the hook still runs.
- Shape 0 shows the default, marks it, and stamps the clock.
- A shape past the table leaves the current image.
- A focus increments the call count before the hands are copied, and clears a session mark of 1.

**Non-Goals:**

- The focus method body.
- The rest of the session-mark leave path.
- Drawing the image through the platform cursor.

## Decisions

1. The table, the default image, and the current image are fields on `MatchGate`. Tests supply synthetic images.
2. `TGUIForm` moves from 3% to 4%. `SG_PlayerControls` stays 8%. The UI section stays 0%. `XA_Arena` stays 34%. The game-modes section stays 0%. The runtime section stays 4%.

## Risks / Trade-offs

- Counting the focus call is not its method. The body was not a fixed function in the image.

## Migration Plan

- Restore applies the shape. The follow invokes the focus first. The ten words stay as they are.

## Open Questions

- None for the record index and the show rule.
