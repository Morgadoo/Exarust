## Why

Exarust looks a clip up by a moveset name: the first moveset of that
name, then its parents, each again the first of its name. The original
does not. `humanoid.rsq` holds a tree in which creature variants reuse
the human names (the golem's, the demon's and the ogre's own "One
Handed", the golem's and the ogre's "1H & Shield", three "Polearm"s), and
the original builds each character's clip table from that tree: the
race's own moveset chain overlaid from the root, then the moveset the
held items ask for, found under every ancestor of the race's moveset,
then the "Torch" overlay. With the name lookup the creature variants are
unreachable, an armed creature plays the human sword clips alone, and a
torch beside a shield cannot be expressed (L1 left both as "not yet").

Phase 4 (Combat and items), milestone M4 "first duel": master-plan
change M15 of lane B (animation), after L1
(`choose-the-moveset-from-held-items`), on which it builds.

## What Changes

- The moveset tree: a moveset's parent is the nearest moveset before it
  whose name is exactly its parent name; a moveset without a parent name,
  or with none of that name before it, is a root. Children keep the
  file's order.
- A character's clip table, as the original's motion constructor builds
  it from a base moveset (the race's own: `Default` for humans) and a
  config word from the held items:
  1. the first moveset named like the base (ignoring ASCII case) and its
     ancestors, overlaid from the root down (each clip a moveset defines
     replaces the table's);
  2. the weapon moveset of the config's low 16 bits (One Handed 0x1,
     1H & Torch 0x3, 2W Light 0x4, 1H & Shield 0x100, Shield C1 0x200,
     Shield C2 0x300, Two Handed 0x10, 2H Light 0x20, Greatsword 0x30,
     Polearm 0x40), searched under each ancestor from the root down to the
     base, children first and then each child's descendants in order;
     every one found is overlaid with its own ancestors below the ancestor
     it was found under;
  3. the overlays "Torch" (mask 0x10010000) and "Flaming" (0x10020000),
     each asked for when the config shares any bit with its mask (the two
     masks share 0x10000000), searched among the children of the last
     weapon moveset found and of its ancestors below its ancestor, then of
     the earlier ones; the first found is overlaid.
  A base the motion set lacks gives no table (the original logs "Base
  motions not found"). Tables are assembled once per base and config.
- The held-item choice (L1) returns that config instead of a moveset
  name: the fists no weapon moveset, a main-hand light "One Handed" with
  the Torch overlay (0x10010001), a shield with a light in the main hand
  its shield config plus the Torch overlay. The body's animator keeps its
  race's moveset as the base and takes the config; the clip playing goes
  on in the new table.
- Every clip lookup in Exarust goes through the table (the body tick's
  clip clock, the gait floor, the get-up choice, the per-tick trace, the
  swing progress, the frame-clocked bodies).
- Debug helper: `exa-tool clips <base> [config]` prints a character's
  clip table, action by action, with the moveset each clip comes from;
  `exa-tool rsq` numbers the movesets and names the one each parent
  resolves to.
- Docs: RSQ.md (Motion set: the tree and the table; Greatsword's parent
  is 2H Light), PLAYER.md (Moveset from held items), PLAN.md (Phase 4),
  crate READMEs.

Evidence:
- REA on `Exanima.exe` 0.9.5 and its disassembly: the motion
  constructor (cache lookup by name and config; base lookup, ASCII case
  folded over the 16-byte name; the copy of the root's table and the
  overlays root first; the weapon table of ten 20-byte records; the
  per-ancestor search, children first then recursive; the path overlay
  below the ancestor; the overlay records "Torch" 0x10010000 and
  "Flaming" 0x10020000 tested with AND; the single overlay per name; the
  cache append), the motion-set loader's tree pass (parent = the nearest
  earlier moveset whose 16 name bytes equal the parent's, children
  appended in order), the clip copy (only clips with keys), and the two
  held-item choosers that call it with the race record's own moveset
  (`context/re/master-animation.md` §0.8, M15 findings).
- Corpus: the 32 movesets of the installed `humanoid.rsq` (every name
  zero-padded after its terminator, so name comparison equals the 16-byte
  comparison), the races' own movesets in `races.rdb`, every race base
  against every held-item config compared with the name lookup
  (`exa-tool clips`, the corpus test).

Out of scope:
- what sets the shield styles (C1/C2) and shields themselves; Exarust's
  items hold no shields or second weapons yet;
- the weapon-ignition path that asks for the Flaming overlay (no
  "Flaming" moveset ships; its shared bit gives a flaming one-handed
  weapon the Torch clips), and the character-record chooser of the
  character view and factories;
- `Hands` on placed characters: level characters keep their race's
  moveset with no held-item config until they hold items (lane D);
- clip clocks, blends and starts (M14, C10), which this change leaves as
  they are.

## Capabilities

### New Capabilities
(none)

### Modified Capabilities
- `motion-playback`: the moveset tree and the clip table replace the
  name fallback.
- `combat`: the held items ask for a moveset config looked up under the
  race's moveset.
- `asset-inspection-cli`: `exa-tool clips`, numbered movesets in
  `exa-tool rsq`.

## Impact

- `exa_formats::rsq` (`MovesetConfig`, `ClipTable`, `MotionSet::parent_of`,
  `assemble`, `table_clip`; `clip` takes a base and a config)
- `exa_anim` (`Animator::config`, `Motions::table`/`clip` with the cache,
  `Motions::new`, `Animator::plays_default_alone`, `DEFAULT_MOVESET`)
- `exa_character::held` (`HeldMoveset::config`, `choose` without a base)
- `exa_game` (`limp`, `balance`, `trace`, `player`, `torch`:
  `TorchHand::config`), `exa_combat` (swing progress, the stance fallback)
  — one-line lookups in other lanes' files
- `exa_tool` (`clips`, `rsq`)
- `docs/formats/RSQ.md`, `docs/PLAYER.md`, `docs/PLAN.md`,
  `crates/exa_formats/README.md`, `crates/exa_anim/README.md`,
  `crates/exa_character/README.md`, `crates/exa_tool/README.md`
