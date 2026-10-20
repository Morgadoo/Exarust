# Proposal

## Why

`XA_Arena` (57.9 KB, rated 5%) carries the arena's 13 match templates, its five tournament formats, the rank and XP rules, the cash and XP prizes, wages, recovery, the prize pools and the seven-day/four-week calendar. Only the log and pick helpers were modelled; the tables and career formulas were missing.

## What Changes

- The six standard, seven ranked and five tournament templates with their exact names, ids, flags, teams, fighters, rounds, cash, XP, slot limits and descriptions.
- The name lookup, including id 11's lower-case "beast".
- Base-12 packed money, the slot-scaled cash prize, the `1 + 0.25·slot` XP pot, the survivors' XP share, the manager's +50% and the per-fighter split.
- Rank points (`1 + Σ xp/5000`), the loadout tier (`(rank−1)/2`, capped at 5) and the XP gain (`min(total/rank, 5000)`).
- The flat 12 hire fee, the four wage classes, daily recovery with and without a physician, and the unfit threshold.
- The 7-day week phases, the 4-week season, the 18 weekly prize ids and the four season armour sets.
- The 19 company names and profile bytes.

## Capabilities

### New Capabilities

### Modified Capabilities

- `arena-career`: the match table and the career formulas.

## Impact

- `exa_game::arena`, from `MatchTemplate`, `STANDARD`, `RANKED`, `TOURNAMENTS`, `template`, `lookup_name`, `unpack_money`, `pack_money`, `cash_prize`, `prize_pot`, `xp_award`, `xp_split`, `rank_points`, `loadout_tier`, `xp_gain`, `wage`, `recover`, `is_fit`, `phase`, `week_of_season`, `TOURNAMENT_PRIZES`, `SEASON_PRIZES`, `COMPANIES` and `POINT_LIMITS`.
- `docs/PLAN.md` and the generated `docs/COMPONENT-MAP.md`: `XA_Arena` becomes **15%** in the source rating. The headline rises.
- Evidence: the 18 records at `0x1002bc6d0`, `0x1002bc3c0` and `0x1002bc970` (read byte-for-byte); the rank/XP formulas at `0x10017cdb0`/`0x10018d0a0`; prizes at `0x100187840`; cash at `0x10018c2d0`; wages at `0x10018d840`; recovery at `0x10018c130`; the calendar at `0x100193050`; the prize tables at `0x1002bcba0`/`0x1002bcc40`; the companies at `0x1002bbf30`. No new corpus files.
- Out of scope: the match flow, opponent generation, the live board, the shop/arsenal, tournament resolution and the UI.

## Phase

Phase 7, arena. The match and career rules.
