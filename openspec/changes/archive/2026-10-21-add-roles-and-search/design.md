# Design

## Context

The original keeps one role database, a role list and two kinds of
relation tables on every character record, and an AI per character whose
idle state is installed by applying its roles. The search is an idle
activity object created from a registry by id.

## Goals / Non-Goals

**Goals:**

- Parse every shipped role; keep the entries' sections as bytes reached
  by the original's section walk.
- Reproduce the role-list, relation and attitude rules with the original's
  branch order, constants and float/double steps.
- Run the search's state machine against a scene and a searcher interface,
  returning its deeds for the caller to carry out.
- Give the running game's level characters roles, relations and a brain.

**Non-Goals:**

- A script engine, power overrides, the group blend, path following in
  the running game, and item instances for the search.

## Decisions

1. Roles are a component holding the list newest first. A v7 record's list
   is kept as saved (the original does not re-expand it); an older list is
   added again so its parents return.
2. An invalid role id behaves as the original's empty fallback role: no
   flags, no entries, weight 0 (so it is still added, and stops the parent
   walk).
3. Removing an explicit role removes it even when other roles also counted
   it; this is the original's behaviour.
4. The attitude blend is `v · s + (1 − s) · base` with `s` the role weight
   share computed in double and stored in float, as the original does.
5. The search's world is two parts: a `Scene` of plain data (objects,
   characters, the room grid, item stats, containers, open windows) and a
   `Searcher` trait for the brain calls (wait, attend, plan, look). Its
   item effects are returned as deeds.
6. The comment phrases are Exarust's own wording; the table sizes (six and
   four templates, five and four drawn), the placeholders, the start marks
   (8 and 5) and the no-repeat rule are the original's. The original's
   lower-casing never reaches a name's last character; that is kept.
7. The approach clamp's last test reads a vector whose first component the
   original overwrites; Exarust uses both components (labelled deviation).
8. Wiring: the brain systems run before the duellist; the fight start no
   longer engages level characters, whose rating does.

## Risks / Trade-offs

- The locale a one-character entry compares with is taken to be the
  current locale's id in `locales.rdb` [guess].
- The player's id (1) is Exarust's; the new character's id is not traced.
- Duelling level characters makes hostile human-framed NPCs attack once
  their hostility builds; non-human frames keep standing.

## Open Questions

- The meaning of role flag `0x100`, entry sections 1, 8 and `0x10`, and
  the variable type tag (corpus: none present).
- The phrase-name accessors (two are called; Exarust uses one name).
- What the model byte the wear room reads is (REA: the object database's
  model section, not decoded).

## Docs

`docs/formats/OTHER.md` (Roles, Locales) is extended.
