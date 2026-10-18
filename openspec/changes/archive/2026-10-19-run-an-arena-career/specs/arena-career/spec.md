# Spec Delta

## Purpose
The arena career: a company's money, who it can hire, and what a standard match pays.

## ADDED Requirements

### Requirement: New career
A new company SHALL start with 343 copper. That total SHALL split into gold, silver and copper at 12 to 1.

#### Scenario: Seed
- **WHEN** a company is created
- **THEN** it holds 343 copper, which is 2 gold, 4 silver and 7 copper

### Requirement: Roster
A company SHALL hire at most seven combatants, and at most one merchant, one physician and one trainer.

#### Scenario: Caps
- **WHEN** seven combatants and one physician are hired, then one more of each
- **THEN** both further hires are refused

### Requirement: Match
Entering a standard match SHALL subtract its fee from the company's copper. A win SHALL add the prize. Experience SHALL be the template amount times the share of opponent health dealt, and half again when a trainer is hired. A loss SHALL not add the prize.

#### Scenario: Duel
- **WHEN** a company with a combatant enters a duel and loses after dealing all of the health, then hires a trainer, enters again and wins after dealing half
- **THEN** the first result has paid the fee and credited 1000 experience, and the second has added the prize and 750 experience
