# npc-roles Specification

## Purpose
What a character is and how it stands with each other character: the role database, a character's roles and relations, the attitude that follows, and the scripts an encounter runs.

## Requirements

### Requirement: Role database
The reader SHALL parse every record of the installed `charroles.rdb` (49 roles, each ending exactly at its size) into its flags, name, parent, weight, init script, variables and entries. A record whose size passes its payload, an entry under 20 bytes, entries past the record, or a wrong magic SHALL be an error.

#### Scenario: Skeleton
- **WHEN** a record of weight 50, parent 2 and the inherit flag holds attitude entries `0x9C` for role 1 and `0x50` for role 9, and another record's entry count passes its end
- **THEN** the first inherits with a share of 0.5 and attitudes −100 and +80 at tolerance 0, and the second is an error

### Requirement: Entry sections
An entry's sections SHALL be walked in the order of bits 1, 2, 4, 8, 0x10 and 0x40, a dialogue sized by its second word and the others by a length word. The corpse section SHALL be found after the sections of bits 1, 2, 4 and 8 only. A section running past its entry SHALL be absent.

#### Scenario: Dialogue, encounter, condition
- **WHEN** an entry holds a dialogue of 12 bytes, an encounter script and a condition script, and another holds a corpse script after a first section
- **THEN** the encounter and condition scripts are found after the dialogue and the corpse script after the first section

### Requirement: Locale
A level's locale SHALL be the `locales.rdb` record whose 20-byte name is the level's name without extension, ignoring case. An unknown name or a bad database SHALL have none.

#### Scenario: Exanima01
- **WHEN** `exanima01.rfc` and `arena7.rfc` are looked up in a list naming `Exanima01` under id 2
- **THEN** the first is locale 2 and the second has none

### Requirement: Adding a role
Adding a role SHALL add it explicitly and, while the role just handled inherits, its parent. A parent already held SHALL count one more; a new one SHALL be added as inherited. The new roles SHALL go in front in the order child, parent, grandparent, each with its role's weight share. Adding a role already explicit SHALL change nothing.

#### Scenario: Golem Sentry
- **WHEN** Golem Sentry (weight 0) is added, added again, and Golem is added explicitly
- **THEN** the list is Golem Sentry, Golem (count 1, share 0.5), Construct (count 1); unchanged after the second add; then Golem explicit with Construct at 2

### Requirement: Removing a role
Removing a role SHALL remove only an explicit one. Each inherited parent SHALL then be removed when its count is under 2, and otherwise counted down.

#### Scenario: Golem Sentry goes
- **WHEN** Golem Sentry, then Golem, were added, and Construct and then Golem Sentry are removed
- **THEN** removing the inherited Construct changes nothing, and after Golem Sentry only Construct is left, at count 1

### Requirement: Roles of new characters and records
A new story character SHALL be Player, Human and its background (0 the unknown, 1 and 2 the proctor, 3 the villager). An arena fighter SHALL be Unaligned. A version 7 record's list SHALL be kept as saved and an older one added again in its order.

#### Scenario: The unknown and a legacy skeleton
- **WHEN** a story character of background 0 is made, and a version 6 record lists Skeleton
- **THEN** the character is the unknown, Human (counted once) and Player, and the record gives Skeleton and Undead

### Requirement: Relation tables
Global characters SHALL be kept by their id's low 28 bits. Level characters (top bit set) SHALL be kept in the current locale's table, chosen when first needed: an existing table of that locale, else a new one when none is current, stored unless its locale is 0. Id 0 SHALL have no relation.

#### Scenario: Locales
- **WHEN** relations about `0x10000005` and `0x80000005` are created in locale 2, locale 3 is then chosen, and a level relation is created in locale 0 by another character
- **THEN** the first is found as 5, locale 2 stays current and holds the second, and the locale-0 table is not stored

### Requirement: Relation roles and familiarity
A relation role SHALL be added once and gain each value once; with no value an existing one SHALL be left. The last attitude of a global partner missing from the table SHALL move its relation from the stored table of its former locale; a partner still missing SHALL get a new relation and the answer 0.

#### Scenario: Values and a former level character
- **WHEN** Assailant is added toward a partner with values 12, 12 and 13, and a partner formerly `0x80000003` of locale 2 is asked its last attitude
- **THEN** the values are 12 and 13 at share 0.5, and the former relation answers from the global table

### Requirement: Attitude order
The attitude of A toward B SHALL start at the familiarity. A's relation roles toward B SHALL decide first: the first relation role with an attitude entry for the partner. Otherwise A's roles, newest first, and their attitude entries in order: an entry for one character SHALL match that character in its locale; any other SHALL match one of A's relation roles toward B or one of B's roles.

#### Scenario: Truce first
- **WHEN** a skeleton with a Truce toward a human rates it, and an entry of XA_Derrin for character `0x80000008` of locale 2 is tried in locales 2 and 3
- **THEN** the Truce decides, and Derrin's entry holds only in locale 2

### Requirement: Attitude value
The first match SHALL give `attitude · s + (1 − s) · familiarity`, with `s` its role's weight share, and its tolerance. No match SHALL give the familiarity and tolerance 0. The result SHALL be kept as the relation's last attitude.

#### Scenario: The shipped matrix
- **WHEN** a zombie, a skeleton and a demon rate a human, a demon rates a demon, a skeleton rates a golem, a human rates a demon, and a skeleton rates a human of familiarity 20 before and after a Truce
- **THEN** they are −20, −50, −100, +100 at tolerance 60, +30, −50, then −40 and 10 at tolerance 100

### Requirement: Encounters
An encounter SHALL create the relation at familiarity 0 or renew its time. It SHALL return, in order, every encounter script of A's roles that is for anyone, for B in the current locale, or for one of B's roles, once per matching role.

#### Scenario: A golem meets a human
- **WHEN** a golem role with encounter scripts for anyone, for character `0x80000002` of locale 2, for Human and for Construct meets human `0x80000002` in locale 2, again later, and in locale 5
- **THEN** the scripts for anyone, the character and Human are returned, the time is renewed, and locale 5 returns two

### Requirement: Entry finder
The entry finder SHALL take, for each of A's roles newest first, the role's first entry aimed at one of B's roles, then at one of A's relation roles toward B, and SHALL give the requested section of the first such entry that carries it, read through the section table, so the corpse section SHALL never be given.

#### Scenario: An attitude entry hides a condition
- **WHEN** one role's only entry for the Player is a condition, and another role's first entry for the Player is an attitude followed by a condition
- **THEN** the first gives its condition and the second gives none

#### Scenario: No corpse section
- **WHEN** a role's entry for the Player holds a corpse script and the finder is asked for the corpse section
- **THEN** it gives none, while the entry's corpse accessor finds the script

### Requirement: Level characters carry roles
Each living placed character SHALL carry its record's roles, empty relations and a brain that applies the roles when it appears. Every frame the brain SHALL rate the player from both sides' roles and keep that attitude, tolerance and its record's traits in its contact rating. The player SHALL carry the new-character roles in the story and the arena roles elsewhere.

#### Scenario: A patrolling skeleton
- **WHEN** a skeleton holding Activity Patrol with traits 20 and −30 appears while the player is Player, Human and the unknown background
- **THEN** it patrols and rates the player −50 at tolerance 0 with traits 20 and −30

### Requirement: Sleepers wake and hostile characters duel
A sleeper rating the player above a notice SHALL lose the sleep role and have its roles applied again. A level character SHALL duel the player once its brain's target pass enters combat against the player, and the fight start SHALL NOT engage it.

#### Scenario: A sleeper's concern rises
- **WHEN** a sleeping skeleton's concern rises to 1.5
- **THEN** it drops the sleep role and roams

### Requirement: Corpse scripts
Handling a dead character SHALL give, for each of its roles in list order (invalid ones skipped), the corpse section of the role's first entry aimed at the Player with no condition, when that entry carries the corpse section; a role whose first such entry lacks it SHALL give nothing.

#### Scenario: First Player entry only
- **WHEN** a corpse holds a role with a corpse script for the Player, one with a corpse script for another role, and one whose first Player entry is an attitude entry followed by a corpse entry
- **THEN** only the first role's script is given

### Requirement: Interaction condition
The interaction condition of A toward B SHALL be the condition section of the entry the entry finder gives for that section.

#### Scenario: A condition entry
- **WHEN** A holds no role, then a role with a condition entry for the Player
- **THEN** there is no condition, then that entry's script

### Requirement: Roles script functions
The roles unit SHALL register `StartDialogue` and then `ChangeRole`; `ChangeRole` SHALL report that the function does not exist.

#### Scenario: Registration
- **WHEN** the registration is listed and `ChangeRole` is called
- **THEN** the names are `StartDialogue`, `ChangeRole`, and the error reads `Function "ChangeRole" does not exist.`

### Requirement: Level relation removal
Removing a relation role toward a level character while no level table is current SHALL select the locale's table first.

#### Scenario: No current table
- **WHEN** a relation role is removed toward a level character with no table current
- **THEN** nothing is removed and the locale's table is current
