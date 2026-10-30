# Design

## Faithful quirks kept

- The retention conversion branches on an ordered `r > 0`, so a NaN
  retention stores zero rather than a NaN.
- The block descriptor's vertical half extent is `(hi − lo)·0.5 + 1`
  (centimetres), with no separate minimum.
- The transparent-alpha test widens the f32 alpha to double before
  comparing it with the double 0.01, so the f32 `0.01` is transparent.
- The listener's point is `camera + (followed − camera)·0.5`, in the
  engine's subtract, multiply, add order.
- The new-sector path registers the scene node's children only while the
  sector holds no static nodes; the shared path never touches the first
  sector's scene word or node.

## Departures

- Registration is the caller's: the new-sector path returns the children
  to register at once, as the update returns pending nodes.
- The terrain kd-tree's attach computes the box from a centre and half
  extent the caller passes; the original reads them from the mesh.
- The snapshot callback is a method with no caller until the sector's
  per-frame node pass is modelled.
