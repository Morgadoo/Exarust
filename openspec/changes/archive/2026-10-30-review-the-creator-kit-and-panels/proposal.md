## Why

The charcreate6 lane added the creator's kit routes, equipment panels, skills page side effects, continue page texts, icon sheet hook and voice choices. Review against the original's assembly found the kit entry modelled with three colours and two separate dyes (the original reads three tint bytes and four colour-layer bytes), the apparel panel left unbound for a named story background, the skills page's panel state fixed to first-visit constants, the continue page's texts driven by a caller-chosen save instead of the list's first entry, and raw in-memory layout offsets in the code.

Phase 7 (game modes and menus) in docs/PLAN.md.

## What Changes

- A kit entry carries its variant, three tint bytes and four colour-layer bytes. A made instance writes the four layer bytes, each layer marked with five, and the second and third tint bytes as dyes where set; an alias's target takes the variant and all three tint bytes. The kit match compares the four layer bytes.
- The apparel panel is bound as the current equipment form as soon as it is made, before the story background's early return; an existing panel is bound without consulting the table.
- The skills page hides the second preview panel only when it exists and flags the skills panel only when it makes it, rebinding an existing panel's record.
- The continue page selects the save manager's first entry, shows its display string and its name; an empty list clears both.
- The preview list's raw entry layout constants are removed.

## Evidence

The kit resolver, alias resolver, kit match and apparel/weapon panel builders, the skills page, the continue page and its pick handler, the preview push and the icon sheet routines, read in the charcreate pack's assembly listing.

## Out of scope

Wiring the panels and the continue texts into the running creator and save chooser; the object database's sub-record reads; the icon sheet's image decode and Kaiser filter; the alias resolver's failure returns when the entry's own record is missing.
