# Design

The character rules stay pure functions in `exa_assets::content_stream`
because Exarust's level loader does not drive the content stream; when it
does, the streamed builder calls `streamed_size_words` with the placement's
condition and then `character_scales`, and the request builder calls
`character_scales` on the stored words.

The reader dispatch models the original's two sources: a name without a dot
is a provider's image (the texture packs' bare names), decoded by its
magic; a name with a dot goes to the extension registry, which holds only
"rfi". The extension is the text after the last dot, empty when it would
not fit the original's 16-byte field, and compared case-insensitively.

Grey specular and emissive maps reuse the diffuse mechanism: the record's
grey bit becomes a material flag the shader reads to repeat the red
channel. Streamed fire textures bind the placeholder until the upload and
are rebound like the other materials, so the flame setup no longer decodes
on the main thread.
