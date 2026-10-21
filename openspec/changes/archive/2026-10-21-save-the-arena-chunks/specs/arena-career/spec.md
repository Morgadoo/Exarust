## ADDED Requirements

### Requirement: Company chunk
The company chunk SHALL hold the name, a second string, the 28-byte emblem record, three words, one unused word and sixteen record words; a string SHALL be a word count and its bytes. Reading SHALL skip the unused word, writing SHALL store 0 there, and a payload that ends inside a field SHALL be an error.

#### Scenario: Ash Company
- **WHEN** a company named Ash Company with emblem bytes 52 and 81, three matches won and 343 copper spent is written, framed, read, read with the unused word set, cut one byte short, and a string count of 0xffffffff is read
- **THEN** the payload is 127 bytes starting with the count 11, the frame starts with the id bytes 00 00 dc a0, both reads give the company back, and the short and oversized payloads are errors

### Requirement: Day chunk
The day chunk SHALL hold today's matches, copper spent and earned, a count of special matches with each one's template id, rank, flags and an unused word, a word beside the log, and the log's count and twenty-byte records. A negative count SHALL read no entries.

#### Scenario: Two specials and a record
- **WHEN** a day with two specials and one log record is written and read, cut to 20 bytes, and a count of −1 is read
- **THEN** the payload is 76 bytes and reads back, the cut payload is an error, and −1 reads no specials

### Requirement: Tournament chunk
The tournament chunk SHALL hold the week's prize word, 0x120 bytes of event rows, sixteen rival company words, six events of four words, four week words and eight prize history words. A field whose first company is 0 SHALL ask for a new field.

#### Scenario: A week in progress
- **WHEN** a tournament with prize 4, company 17 first, an event row byte, the last event Elimination and a history id 1172 is written and read, and cut to 100 bytes
- **THEN** the payload is 500 bytes and reads back without asking for a field, an empty field asks for one, and the cut payload is an error
