# Proposal

## Why

Phase 6 can lie dormant. A sleeper does not yet wake when it sees a contact: drop the sleep role and be knocked out at the current clock.

## What Changes

- Flags that are not exactly the dormant flag, or no contact, leave the sleeper as it is.
- A contact drops role 44 and knocks the body out at the current clock.
- The condition becomes down. The dormant flag stays and the waking flag is set. The statue mark clears and gait is 0.
- Tone is 0 and the pin gain is 0.08. The sleep countdown is raised to 256.

## Capabilities

### New Capabilities

### Modified Capabilities

- `npc-idle`: waking a sleeper that sees a contact.

## Impact

- `exa_ai::idle`, calling the body's countdown wake.
- `docs/PLAN.md`, `docs/COMPONENT-MAP.md`.
- Evidence: the sight check and the knock-out it calls with no delay. No format field changes, so the corpus step is a skip.
- Out of scope: the tone rise after the knock-out, and the path search.
