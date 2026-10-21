# Design

## Context

The week's six events are the roster rows `exa_game::arena` keeps (an
entrant count and a winner team). The board's Begin and the manage
window's day button test them. Rule page:
[`docs/formats/OTHER.md`](../../../docs/formats/OTHER.md) (Arena tables).

## Goals / Non-Goals

**Goals:** the gates as plain functions over the rows; the health bar.

**Non-Goals:** the windows.

## Decisions

1. Counts are compared as signed words, as the original does.
2. The day button is named by its use (it gates the manage window's day
   control); the label is inferred from the window, not from a string.

## Open questions

- Which control the day gate drives: the manage window's controls are
  read when that window is built (REA on its form).
