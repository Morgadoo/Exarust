# Spec Delta

## ADDED Requirements

### Requirement: A duellist's foe is its target's fighter
A brain-bearing duellist aimed at a character SHALL take as its foe the fighter of that character (the aimed entity when it fights, else the fighter it carries), so the perception's contact, the target pass, the entry into combat and the combat state's contact all name that fighter.

#### Scenario: Aimed at the player's root
- **WHEN** a brain whose roles rate the player hostile duels the player's root, whose child is the player's fighter
- **THEN** the target pass enters combat against that fighter, the duellist engages, its combatant targets the fighter with a hostile contact, and the root has no contact
