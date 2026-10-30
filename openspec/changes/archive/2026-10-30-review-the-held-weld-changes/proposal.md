## Why

The motilepart10 lane added the collapse's and death's changes to a held item's world welds, the auto-parry gate's creature-body condition, the collapse's groan and the sole bit a held shield or torch loses. The review checked each against the assembly: the gate, the death's strength-1 write, the groan's draws and the sole bit are right, but the collapse slack also overwrote the link's smoothed length (the routine writes the slack word alone), both changes were applied once for an item's life instead of each time the routines run, and the groan had no caller. The earlier spec said death leaves every grip link alone; the death routine does stiffen each held item's soft weld.

Phase 4 (Combat and items) in docs/PLAN.md, TMotilePart.

## What Changes

- The weapon's soft (parry) weld is built only when the holder is neither in the heavy-claws state nor a creature body (the body rule flags' bit 0). Live in the equip and off-hand attach.
- A holder that starts collapsing gives each item it still holds a 10 cm soft-weld slack (the slack only); one that dies makes the soft weld strength 1. Each applies on the tick the holder enters the condition. Live in the grip runtime.
- A collapse's start asks the character's voice for line 2 at volume 0.15 + 0.1·second draw and pitch 0.8 + 0.1·first draw, both in double. Live through the pain voice's sound path.
- A held shield or torch's body loses the foot-sole bit.

## Evidence

The motilepart pack's assembly: the world-weld builder (the gate bytes and the rule word's bit 0), the collapse routine (the two draws, the voice call, the fall-class gate and the slack write into each held item's soft-weld link), the death routine (the strength write into the same link when its index is above zero), the shield and torch attaches; the runtime link's field use in the grip-mode routine (slack and smoothed length are separate words).

## Out of scope

The fall-class gate on the collapse slack (Exarust's collapse always starts the fall motion), what enables the disabled world welds, the per-array unlinks of the hand-empty and child-list routines, the global parry switch (always on in retail).
