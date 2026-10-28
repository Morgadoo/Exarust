# Design

## Loader wait

After a decode the original checks the global upload count under the
upload lock: if it is above zero and the record is not promoted, it
unlocks, sets its state to "waiting for uploads" and suspends its own
thread until the upload pass wakes it; it then queues its upload. The
upload pass wakes the loader after every pass unless the loader's idle
flag is set (its ring was empty), and a queued request clears that flag.

Exarust's executor decodes on Bevy's task pool and collects results on the
main thread, so the held record's upload is queued at once (behind the
earlier ones, so the order is the same) and `TextureStream` remembers the
held key: `can_decode` is false while it is set and a pass clears it. The
held record keeps its in-hand bit in the original, so a priority load finds
it and drains it; a record already in the upload list has had its bits
cleared by the loader's next look and is not found.

## Placed object flags

The placed-object reader merges the object flags under `0xB80000` into its
root node and sets `0x80000` on the root for placement bit `0x10`. The prop
reader does the same and then calls a walk that sets `0x80000` on every
node below; the placed-object reader does not. Exarust keeps the flags
parsed (`PlacedObject::is_static`) and registers the bodies as before,
until the effect of the root flag on a model's bodies is traced.

## Cube uploads

The upload routine picks the GL target from the creation word and then
uses the same detail-level and chain logic for every target, so a cube
uploads six faces from the detail level, each with its levels. Exarust
builds a cube view over a six-layer image; the environment map uses the
same builder.
