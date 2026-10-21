# Proposal

## Why

The creation screen labels a record by its appearance band: human male or female, HAM, OM or ML, and nothing for any other band. Exarust has the record move but no labels.

## What Changes

- Bits 8 through 15 give the band and bit 0 splits the human pair.
- Band 0 is HF when bit 0 is set and HM otherwise.
- Band 0x200 is HAM, 0x300 is OM and 0x400 is ML.
- Any other band has no label.

## Capabilities

### New Capabilities

### Modified Capabilities

- `character-creation`: the appearance labels.

## Impact

- `exa_game::create`, from `creation_label` and `CreationLabel::code`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`: `SG_Character_Create` moves from 2% to about 2%.
- Evidence: the 158-byte appearance label: the band mask `0xff00`, the bit 0 split and the literals HF, HM, HAM, OM and ML. No new corpus files.
- Out of scope: the caller that writes the label.

## Phase

Phase 5, character creation. The appearance labels.
