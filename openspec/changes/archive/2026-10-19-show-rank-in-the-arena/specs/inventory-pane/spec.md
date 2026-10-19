# Spec Delta

## ADDED Requirements

### Requirement: Rank and points
Entering arena SHALL turn rank and points on. Entering story SHALL turn them off. Entering practice SHALL turn them off unless the mode being left already has bit 2. The same mode, and any other mode, SHALL leave them. A shown rank SHALL use the skill byte's top three bits: zero is absent, then Aspirant, Novice, Adept, Expert, Master, a dash, and an empty seventh. Shown points SHALL be the low five bits times one half, and zero SHALL be absent.

#### Scenario: Arena opens the rows
- **WHEN** menu 0 enters arena 8, arena enters story 4, story enters practice 2, practice stays practice, mode 6 enters practice, and arena enters mode 1, then a shown skill of 0, 0x21, 0xc0, 0xe0, and 0x1f is read, and a closed gate reads 0xdf
- **THEN** arena opens both, story and that practice close both, staying and mode 6 and mode 1 leave them open, skill 0 is empty, 0x21 is Aspirant and 0.5, 0xc0 is a dash, 0xe0 has no name, 0x1f is 15.5, and the closed gate shows neither
