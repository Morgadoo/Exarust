## Decision

Keep one surface pipeline and branch on uniform flags, as the other program differences already do: no new pipelines, a uniform branch per material, so the frame time stays as it was. The hair maths lives in a WGSL library without imports so a unit test validates it with naga; the composed surface shader is validated by the existing `surface_shader` test.
