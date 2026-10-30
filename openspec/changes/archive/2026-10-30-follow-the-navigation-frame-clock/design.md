# Design

The original stamps each frame with an integer millisecond and keeps the
end of the last tick on the same clock. Exarust has no such stamp, so the
chase camera sums its real frame times in double and rounds the sum once a
frame; rounding the absolute sum (not each delta) keeps the clock from
drifting. The stamp is wrapped to 32 bits so a wrapped clock compares as
the original's unsigned compares do.

The constructor's registrations (the node's rebuild hook, the scene task,
the viewport callback swap) are flags on `NavController`; Exarust has no
viewport GUI lists, so the host performs them. The saved callback is the
viewport's slot as found, so a viewport without one leaves the destructor
nothing to run.

The detach's byte is the controller's redraw byte: the frame driver clears
the scene's and its caller's redraw requests only while it is clear, and
the camera tick clears it. It was first modelled as a viewport geometry
reset, which the listing does not show.

The node-frame pick transforms the ray's origin with the engine's point
transform, whose sum is `(x·c₀ + y·c₁) + (z·c₂ + t)`; glam's
`transform_point3` adds the translation last, so the sum is written out.

The scatter's cell side is `1.3 × 1/√2` in double with the original's
constant (one unit in the last place below the correctly rounded 1/√2),
narrowed once: `0x3F6B533C`, one unit above the single-precision product.
The 21 035-point set is unchanged by it; the test pins the bits.
