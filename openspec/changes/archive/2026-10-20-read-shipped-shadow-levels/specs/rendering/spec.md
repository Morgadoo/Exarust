# Spec Delta

## MODIFIED Requirements

### Requirement: Shadow-bias levels
The system SHALL give terrain and tile-grid floors shadow-bias level 3 and other grid set pieces level 1, and every other mesh the level its file stores in bits 16–18 of its render flags (level 0 when bit 19 is set). A surface of level v > 0 SHALL compare shadow distances 0.995·k_c/k_r short, with k_r = 1 − 0.005·v and k_c = k_r − 0.01.

#### Scenario: Torch light grazing a floor
- **WHEN** a torch lights a terrain floor at a grazing angle
- **THEN** the floor shows no acne from its own shadow

#### Scenario: Slab under painted ground
- **WHEN** a broken-floor slab of shipped level 2 lies 1–2 cm under the terrain and a torch lights the terrain over it
- **THEN** the terrain over the slab is lit, not shadowed by the slab
