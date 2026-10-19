# Proposal

## Why

Phase 6 knocks a sleeper out when it sees a contact. The tone stays 0. It does not yet rise by 0.001 a tick after 1500 ms, or finish when the clip base reaches 1.

## What Changes

- While the waking flag is clear, a tick changes nothing.
- Through 1500 ms the tone and stored target stay 0, the clip base is 0.4, and the condition is down with both flags set.
- At 1501 ms the tone gains 0.001, or 0.002 while getting up at phase 0. The stored target stays 0. The dormant flag clears unless held.
- At 8000 ms the clip base stays. At 8001 ms it gains 0.01. At 1 the tone is 1, the target is the stance factor, and the flags clear unless held.

## Capabilities

### New Capabilities

### Modified Capabilities

- `npc-idle`: the tone rise after a knock-out.

## Impact

- `exa_ai::idle`, writing the body's tone.
- `docs/PLAN.md`, `docs/COMPONENT-MAP.md`.
- Evidence: the waking tick. No format field changes, so the corpus step is a skip.
- Out of scope: the awaken script and the weapon refresh.
