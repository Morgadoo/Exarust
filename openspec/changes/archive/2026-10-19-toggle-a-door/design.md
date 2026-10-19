# Design

## Context

The open command already runs when a signalled wait ends. A toggle is the command an AI or a script sends to one door.

## Goals / Non-Goals

**Goals:**

- Open a latched door first, then choose open or close from how open the door is.
- Wake the door for 256 ticks.

**Non-Goals:**

- An AI deciding to toggle because its path is blocked.

## Decisions

1. Hinge openness is `(angle − closed) · −0.7`. Above 0.7 the aim fraction is 1. At 0.7 and below, including a non-finite value, it is 0. A slow hinge runs 100 ticks and a fast one 60.
2. Slider openness is `1 − travel`. Above 0.7 the slider closes for 60 ticks. Otherwise it opens for 60 ticks and the shut flag clears.
3. A latched door is opened before the choice, so a plain slider's travel becomes 0.6 and the choice then sees openness 0.4.

## Risks / Trade-offs

- The placed door does not yet call this. The function is the command.

## Migration Plan

No data migration.

## Open Questions

- None for this slice. The toggle and its openness were read before this change.
