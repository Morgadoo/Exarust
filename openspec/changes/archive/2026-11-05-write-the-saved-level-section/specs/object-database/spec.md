# Spec Delta

## MODIFIED Requirements

### Requirement: Live local tier from a level
A level load SHALL make the live database's local tier the level's `0xDBDB` records as stored (table, flags and blob) and its library the level's `0xDBCD` table, replacing the previous level's and leaving the base and world tiers alone, and SHALL then migrate every record of the tier from its template, whether the level came from the archive or a save's section. A copy made in the local tier afterwards SHALL take the first id the level's records leave free. Every level of `Resource.rpk` with a local database SHALL read as such a tier.

#### Scenario: A copy after the level's records
- **WHEN** `exanima01` is loaded (local ids 1 to 10 taken, 11 free) and a marked template is instantiated
- **THEN** the copy's id is `0x8000000B`

## ADDED Requirements

### Requirement: Database table count
An `.rdb` database SHALL hold `table_len / 16 − 1` records: the table slot after the last entry SHALL not be read as a record. Every `.rdb` of `Resource.rpk` and every level's own database SHALL read so.

#### Scenario: A stale last slot
- **WHEN** a table of two slots holds one record and a slot that would read as a second record inside the data
- **THEN** the database holds one record
