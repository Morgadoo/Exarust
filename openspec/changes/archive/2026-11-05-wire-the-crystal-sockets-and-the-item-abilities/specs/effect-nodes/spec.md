# Spec Delta

## MODIFIED Requirements

### Requirement: Burning blade out and sizzle
A cold socket SHALL put a live flame row out and clear the lit flag; it SHALL NOT choose the moveset again, so the "Flaming" overlay and the body's lit-blade byte stay until the hands are dressed again. A hit struck with a lit blade SHALL play `FireSizzleA` at the contact, at volume 0.3 and pitch 0.8 + 0.2·u.

#### Scenario: The crystal is taken out
- **WHEN** the Power Crystal C leaves a lit blade's first socket
- **THEN** the next held check puts the row out and clears the lit flag, and the Flaming overlay stays

## ADDED Requirements

### Requirement: Lit-blade byte
Lighting a burning blade SHALL choose the moveset again with the Flaming overlay, which clears the holder's lit-blade byte, then raise the byte and set the grip mode again. Every moveset choice made because the hands changed SHALL drop the overlay and clear the byte.

#### Scenario: Re-dressed hands
- **WHEN** a character whose blade lit changes what its hands hold
- **THEN** its moveset is chosen without the overlay and its lit-blade byte is clear
