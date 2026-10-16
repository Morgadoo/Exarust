## MODIFIED Requirements

### Requirement: Race inspection
`exa-tool races` SHALL print every race of the installation's `races.rdb`: its id, name, body model, movesets and health, its flag words (whether clawed and regenerating red), its body parameters (clip-rate base, size, scale, mass divisor, support-pin factor, balance bonus, stance factor, arm relaxation, damping ratio), its stats (unarmed reach, swing turn, recovery, AI attack bonus, perception, held offset, footsteps, voice volume, attack voices) and its four gait rows.

#### Scenario: Retail races
- **WHEN** `exa-tool races` runs against a retail installation
- **THEN** 18 races are printed, Human with stance factor 0.5 and a walk thrust of 88.9 per second

#### Scenario: Ghoul
- **WHEN** `exa-tool races` runs against a retail installation
- **THEN** Ghoul is printed with flags `0x3` / `0x3`, kind `0x100` and clawed, clip-rate base 1.5, unarmed reach 1 m and footsteps `StpSft`
