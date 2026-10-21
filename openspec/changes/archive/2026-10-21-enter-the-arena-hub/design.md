# Design

## Context

The hub entry routine plays a track through the audio environment's play
call and then sets the track's start offset; the hub installs its click and
session hooks. Rule page:
[`docs/formats/OTHER.md`](../../../docs/formats/OTHER.md) (Arena tables).

## Goals / Non-Goals

**Goals:** the music draw, the click and session decisions as plain
functions; the hub's music in play mode.

**Non-Goals:** the windows, the camera, figures in the hub.

## Decisions

1. The music start is a resource the music system applies right after it
   plays the track, as the original sets the offset after its play call.
2. The click hook's object names keep their first byte (0xAA) and compare
   all sixteen bytes; what that byte is in the scene's names is not traced.
3. The side value is kept as a float the caller supplies; what it is in the
   scene (probably the object's place) is not traced.

## Open questions

- What the 0xAA byte of the hub objects' names is: read the scene object
  name loader with REA when the hub's objects are made clickable.
