## MODIFIED Requirements

### Requirement: Running hub clicks
A click that is not a drag SHALL reach the hub's click hook with the hub's busy flag, the mode and whether a hub window is open, and SHALL open the answered window. The hook SHALL compare the stored sixteen name bytes, including the library's leading 0xAA, of the scene node or prop the click picked.

#### Scenario: Clicks in the hub
- **WHEN** an ArenaShop node, an ArenaMach node at side 1 and at side -1, a standing and a downed merchant are clicked, and the shop is clicked while busy
- **THEN** the shop opens the trade window, side 1 the board, side -1 Manage Company, the standing merchant his stock, and the downed merchant and the busy click nothing

#### Scenario: The play view's hub
- **WHEN** the hub is loaded and the flow is in the practice stand-in or the menu, then in practice without the hub, then in story mode with the hub
- **THEN** the hook's mode is the arena's for the first two and practice and story for the others

#### Scenario: The installed hub's scenery
- **WHEN** arenahub is loaded from the installed game and a ray is aimed at a triangle of the ArenaShop node and of an ArenaMach node
- **THEN** each ray's pick names that scene node
