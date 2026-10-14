# Design

## Decisions

- **Prototype on clips, not physics.** Exanima's combat emerges from physical
  bodies; this loop uses clip-driven poses and a blade segment so gameplay
  pieces (input, AI, damage, reactions) exist and can be kept when the
  physical body replaces the clips.
- **Damage ∝ tip speed²** approximates momentum-based damage; armour layers
  and damage types come with the item system.
- **Hit volumes** are the decoded node bounding spheres (scaled to 75 %);
  contact picks the part nearest the blade relative to its radius.
- **Attack window** 35–85 % of the clip, one contact per target per attack.

## Risks / Trade-offs

- Clip-driven strikes repeat exactly, so contacts and damage repeat; physics
  will vary them.
