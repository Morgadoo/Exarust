## Context

The factory table lives outside `exa_formats` (`exa_items` and the factory parser), so the migration cannot look a factory up itself.

## Decisions

- **Caller-supplied pass.** `ObjectDb::migrate_with` takes an optional closure called with the factory name and the weapon body; `exa_items::refresh::factory_pass` is the item side's implementation for a resolved factory. A name no factory has means the closure does nothing, as the original's lookup does.
- **Condition refresh in the migration.** The original runs the weapon condition refresh after the factory step whether or not a factory was found, so it belongs to the migration (`weapon_stats::condition` plus the value rescale shared with the body refresh), not to the pass.
- **Write-back.** The pass writes exactly what the original's factory writes into the body: an unpicked part's selection byte is left as it was on the first-seed path; selection is bounded to the section's ten bytes (a guard; the original has no bound).
- **Name source.** Only the model section visited before the weapon section; the original reads that pointer unconditionally, so a record with its weapon first would fault there; here it is treated as not a factory weapon.

## Risks

None for the running game: the migration still has no caller.
