## ADDED Requirements

### Requirement: Golem suit power
The armour pass's golem-suit registration SHALL power the body's golem-suit flag when the suit's kind is odd and its charge byte is under 0xff, and unpower it otherwise; the suit's worn tick SHALL turn it on while the charge still rises and off once it does not.

#### Scenario: A spent suit
- **WHEN** a golem wears a golem suit whose charge byte is 0xff
- **THEN** its golem-suit flag is cleared at the registration
