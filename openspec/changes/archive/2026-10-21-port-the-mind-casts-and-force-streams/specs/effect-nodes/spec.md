## ADDED Requirements

### Requirement: Blast and torrent geometry
The blast's and the torrent's geometry SHALL use `effectforce_v` with `effectforce_f` and `effecttorrent_f`, bind `ObjMat`, `TexOfs` and `EmiTex`, and draw their passes on the 0.002 row step; their texture offsets SHALL be `(0, r, texture scale)` with the first chunk's radius between its last two ticks, and `(0, scroll, 1)`.

#### Scenario: The torrent's program
- **WHEN** the torrent's geometry class is read
- **THEN** its fragment program is `effecttorrent_f` and its vertex program the tendril's

### Requirement: Blast light
The blast's light SHALL put one point a chunk coloured (0.16, 0.08, 1) times the gain, its strength and the setup's light; the strength SHALL be `(0.6 − c)·c·11.11` under a charge of 0.3 and, thrown, `max(4 − 4.5·f, 0)²` above a spent share of 0.666, and the attenuation 5000 while charging and, thrown, `5000 − 13333·f` under 0.3 and 1000 from it.

#### Scenario: A late chunk
- **WHEN** a thrown chunk has a third of its life left
- **THEN** the attenuation is 1000 and the strength `(4 − 4.5·⅔)²`

### Requirement: Torrent light
The torrent's light SHALL put one point a gathered sample coloured (0.12, 0.06, 1) times the gain, its weight over 4 and `(2 − l)·l·1.2`, attenuation 2000; its samples SHALL merge into the nearest point within 80 cm, or within 160 cm once sixteen are kept.

#### Scenario: A torrent at full level
- **WHEN** a torrent at level 1 lights a point of weight 2
- **THEN** its colour is (0.12, 0.06, 1) times 0.6
