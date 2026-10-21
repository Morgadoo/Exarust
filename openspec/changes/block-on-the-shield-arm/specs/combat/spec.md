## MODIFIED Requirements

### Requirement: Guarded blow
A guarded blow SHALL meet no armour layer and draw no coverage roll. A weapon's parry SHALL deal nothing: the parrying fighter's attack state clears, it is kept from attacking 600 ms unless it has Riposte, and its stance sags by half the impact at the hit, at most 1. A blow on a shield SHALL deal no lasting wound (Shield stamina). Either guard SHALL make a striker smaller than 1.4 recoil. An open guard SHALL keep the flesh wound.

#### Scenario: Parry
- **WHEN** the same cut that wounds bare flesh is parried
- **THEN** the wound and the yellow damage are zero, the fighter's health is unchanged, it may not attack for 600 ms, and the striker recoils

#### Scenario: Tired shield
- **WHEN** that cut meets a shield of block 0.58 with an empty shield arm
- **THEN** the lasting wound is zero and yellow falls to 0.22621973 (bits 0x3e67a625), less than the flesh wound

## ADDED Requirements

### Requirement: Shield stamina
A blow on a shield not parrying SHALL cost the arm's stamina cost·0.8·I, or cost·(0.3x + 0.3)·I with x = clock/0.3 under Synchrony while the holder counters this attack before 0.3 of its clock; the stamina stays at least 0. With x above 0 the arm SHALL leak (1 − S)·(0.4·I + 0.4·x)·(1 − block/2), taking 0.05 of it off yellow (0.025 under a potion), and daze the holder (0.3 − 0.2·S) times the leak.

#### Scenario: A fresh arm
- **WHEN** a cut of impact 0.6746667 meets a shield of block 0.58 on a fresh arm
- **THEN** the arm keeps 0.61678928 of its stamina and yellow falls from 0.25 to 0.24088715

### Requirement: Shield parry
A shield parry SHALL cost the arm cost·0.4·I and leak nothing, lock the holder 600 ms unless it has Counter, sag its stance (0.7 − S/2)·I (at most 1) and daze it (0.3 − 0.2·S)·I; yellow under 0.05 SHALL run the low-health check after any block.

#### Scenario: Parried on the shield
- **WHEN** that cut meets a shield parry on a fresh arm
- **THEN** the arm keeps 0.80839467, the sag is 0.19956821 and the holder is locked
