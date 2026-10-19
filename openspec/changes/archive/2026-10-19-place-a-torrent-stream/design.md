# Design

## Mesh

`torrent_streams` rounds eight tenths of the span to the nearest even integer. `torrent_birth` multiplies the direction by 16, then sets the height to 1.5 times that height minus 30. `torrent_mesh` adds 8 records and asks for 13 groups of each, 12 bytes a group and 8 indices a group.

## Push

`torrent_reach` is the radius times the scale, plus the margin, plus 10. `torrent_hit` adds 0.005 and 0.01 of the age over the sample count. The second channel stops once it is no longer under 1.8. `push_torrent_point` moves a point by 0.01 of one minus its distance squared over the reach squared. On the reach it stays. Past the reach it is missed.

## Open questions

- The index winding, the shell mesh, and a character's impulse are not in.

## Docs

`docs/formats/OTHER.md` gains the stream and the push. `docs/PLAN.md` and `docs/COMPONENT-MAP.md` rate the torrent streams at 20%.
