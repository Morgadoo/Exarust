## Context

How the original builds a character's clip table. The motion library
loads `humanoid.rsq` once. After reading the movesets it links them into a
tree: for each moveset after the first whose parent name is not empty, it
looks backwards from the moveset before it for one whose 16 name bytes
equal the parent name's, takes the first it meets as the parent, and
appends the moveset to that parent's children. So a parent always comes
before its children, the children keep the file's order, a duplicate name
resolves to the nearest earlier moveset of that name, and a moveset whose
parent name matches nothing before it (its own name included) is a root.

After every equip change the held-item chooser computes a config word and
calls the motion constructor with the race record's own moveset name (the
16 bytes at record offset 0x48: `Default` for humans, `Ogre`, `Golem`, …)
and that config. The constructor:

1. returns the cached table when one was built for the same 16 name bytes
   and the same config;
2. finds the base: the first moveset whose name equals the given one,
   ASCII letters compared case-folded over all 16 bytes; if none, it logs
   "Base motions not found" and returns a pointer that is no table (a
   crash path in the original; Exarust plays nothing);
3. collects the base and its ancestors, copies the root's clip pointers
   into a new table and overlays each ancestor below it down to the base:
   every action whose clip pointer is set in the moveset (a clip with at
   least one key) replaces the table's;
4. looks the config's low 16 bits up in a table of ten records (name and
   code): One Handed 0x1, 1H & Torch 0x3, 2W Light 0x4, 1H & Shield 0x100,
   Shield C1 0x200, Shield C2 0x300, Two Handed 0x10, 2H Light 0x20,
   Greatsword 0x30, Polearm 0x40; for each ancestor from the root down to
   the base it searches the ancestor's descendants for that name (its
   children in order first, then, child by child, the same search under
   each child); every hit is overlaid with its ancestors below the
   ancestor it was found under, from the top down, and remembered with
   that ancestor;
5. for each of two overlay records, "Torch" 0x10010000 and "Flaming"
   0x10020000, when the config ANDed with the mask is not zero: from the
   last remembered hit back to the first, it checks the children of the
   hit, then of its parent, and so on up to but not including the
   remembered ancestor, for the overlay's name; the first found is
   overlaid alone;
6. appends the table to the cache.

The chooser's configs (L1's rules, already in Exarust): fists 0; one
one-handed weapon 0x1, two 0x4; two-handers 0x30 (flags 0x521), 0x20
(kind bit 0), 0x10; polearm 0x40; off-hand light 0x3; main-hand light
0x10010001; a shield 0x100/0x200/0x300, ORed with 0x10010000 when the
main item is a light. A second chooser, from a character record's
equipment slots, makes the same words (it gives a main-hand light stance
class 1 rather than 5; out of scope). The weapon-ignition path calls the
chooser with 0x10020000 added.

What the shipped tree makes of it (corpus):
- A human's table is the name chain's for every config L1 can produce;
  only a shield or a flaming weapon with the Torch overlay differ (Walk,
  Cmbt - Idle, Attack Mid and Idle from "Torch").
- An armed creature gets the human weapon moveset first, then its own:
  a golem with a sword plays Default, Golem, the human One Handed and the
  golem's One Handed (#15) over it; an ogre the ogre's (#28).
- Any Herath creature (Herath, Gobbler, Fael, Harvester) with a
  one-handed weapon finds the demon's One Handed below Fael → Demon A01,
  so the Fael and Demon A01 clips come with it: the original's rule, kept.
- The two overlay masks share their top bit, so a main-hand light also
  searches "Flaming" (absent from the data) and a flaming weapon also
  searches "Torch" (found under One Handed: a flaming one-handed weapon
  plays the torch clips).
- The constructor keeps at most four movesets in the base's chain (the
  base and three ancestors), four hits and four path steps in fixed
  slots; the deepest base chain in the data is four long (Demon A01;
  Shield C1). Exarust has no such limit.

The research notes, with addresses, are git-ignored:
`context/re/master-animation.md` (§0.8 and "M15 findings"),
`context/re/combat-stances.md` §1, `context/re/master-plan.md` (B2).

The format page this change extends is
[`docs/formats/RSQ.md`](../../../docs/formats/RSQ.md) (Motion set).

## Goals / Non-Goals

**Goals:** the original's tree, table and overlays for every base and
config; the held items giving the config; every Exarust lookup through
the table; tables built once.

**Non-Goals:** shields, second weapons, flaming weapons, held items on
placed characters, the character-record chooser; clip clocks and blends.

## Decisions

- **The tree and the assembly live in `exa_formats::rsq`.** They are
  rules over the motion set's own data, need no Bevy, and the tool prints
  them. `MotionSet::parent_of` resolves a parent on demand (nearest earlier
  exact name); `assemble` builds the tree once per call and returns a
  `ClipTable` (per action the index of the moveset whose clip plays).
  `MotionSet::clip(base, config, action)` assembles without a cache, for
  tests and tools. The parser is unchanged.
- **Names compare as strings.** The original compares all 16 stored
  bytes; the parser keeps the name up to its terminator. Every name and
  parent of the installed file is zero after its terminator, so the two
  agree; a file with bytes after a terminator would link differently in
  the original (documented, not modelled).
- **`MovesetConfig` is the original's word** (a `u32` newtype with named
  constants), so the tool and the tests speak the same numbers as the
  notes and the overlay masks keep their shared bit.
- **The cache sits in `exa_anim::Motions`**, shared by its clones: a list
  of (base, config, table) behind a read-write lock, as the original's
  list. Every body's lookup reads it; a miss assembles under the write
  lock. `Motions::new` builds the resource; the cache field is private.
- **`Animator::moveset` becomes the base and `Animator::config` the held
  items' word.** `Animator::new(name, action)` keeps working: a name with
  config 0 plays that moveset's chain, which is what the old lookup gave
  for every human moveset, so the character view, the tests and the torch
  bodies keep their clips. The stopgaps that asked "is the moveset
  Default" (the get-up's and a struck fighter's idle) ask
  `plays_default_alone` (base Default and no weapon moveset).

## Risks / Trade-offs

- [Conflicts with C10] C10 edits `exa_anim::Animator` and the clip
  sampling in `limp.rs`; this change adds one field and turns three
  lookups into `motions.clip(anim, …)`. Merges are line-local.
- [A lock on every lookup] An uncontended read lock and a scan of a
  handful of entries per lookup; cheaper than the old name walk.
- [Creature clips untried in game] No Exarust creature holds items yet;
  the creature tables are checked by the corpus test only.

## Open Questions

None blocking. Answered for this change:
- Parent resolution, base lookup, weapon search order, overlay search and
  masks: REA on the motion constructor and the loader's tree pass, with
  the disassembly for the AND test of the masks.
- Which name the chooser passes: REA on both held-item choosers (the race
  record's own moveset); corpus `exa-tool races` for the values.
- Whether the shipped names carry bytes after their terminators: corpus
  (none in `humanoid.rsq`).
