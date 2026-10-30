# Proposal

## Why

The xptrade3 pass on the experience and trade units added the account
experience's settings-store write, the level-start hook, the hook
registration order, the body's technique word read on a level rise, the
player's basket side of a transaction, the apparel copy's quality share and
the shop window's stock binding and preview re-layout. Its review against
the assembly found two departures: the technique word's maps were shifted
by one skill, and the power progress was divided as a signed word.

## What Changes

- A level rise rebuilds the body's technique word from skills 0, 2, 3, 4
  and 5; skill 1 sets no bit.
- The power progress is divided as an unsigned 32-bit word.
- The level-start hook seeds the account pool from the player's carried
  experience and restarts both clocks; the six hooks install in a fixed
  order.
- The account experience is written to and read from the settings store as
  four little-endian bytes.
- A trade also moves the player's basket; an apparel copy's condition byte
  gives the description phrase its quality share; a preview pick lays the
  shop window out on the picked item.

## Capabilities

### New Capabilities

### Modified Capabilities

- `story-experience`: technique word, power progress division, level start
  and hooks, account experience setting.
- `shop-trade`: the player's basket, the apparel quality share, the window
  re-layout.

## Impact

- `exa_game::experience`, `exa_game::trade`, `exa_game::shop_window`,
  `exa_game::hub_windows` (the opener binds the stock), their tests, the
  exa_game README, `docs/formats/SETTINGS.md` and `docs/formats/OTHER.md`.
- Evidence: REA (Ghidra 12.1.4) assembly and objdump of the original. The
  technique routine loads the skill words at the first, third, fourth,
  fifth and sixth of the six 8-byte skill slots, each looping over
  `points / 5000` technique bytes. The conversion sums the progress in a
  32-bit register (zero-extending) before a 64-bit signed divide, so the
  quotient is unsigned. The registration routine stores the new-game,
  level-enter, level-start, tick, kill and pickup handlers in that order.
  The quality reader converts the low two bits and the 5-bit field times a
  double 1/31, negated on bit 7.
- Out of scope: a story runtime driving the hooks, loading the account
  experience at launch, the shop window's own container widgets and
  per-item callbacks, the item transform a taken row goes through, the
  description phrase builder and the weapon variant roll.

## Phase

Phase 7, game modes and content: story experience and the arena shop.
