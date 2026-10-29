// The original's hair program (`shadehar_f`): an anisotropic two-lobe
// highlight along the strands, a soft diffuse with a rim that wraps past the
// terminator, an ambient brightened toward the outline and a reflection bent
// along the strands. Plain WGSL (no imports) so the Rust tests can validate
// it; the formulas are mirrored and pinned in `hair_program.rs`. Rules:
// docs/RENDERING.md ("Hair program").

// The lobe's anisotropic distribution: 1/(π·rx·ry·d²) with
// d = cos² + sin²·((x·h/rx)² + (y·h/ry)²).
fn aniso_d2(ndh: f32, rx: f32, ry: f32, xdh: f32, ydh: f32) -> f32 {
    let cos2 = ndh * ndh;
    let sin2 = 1.0 - cos2;
    let r_x = xdh / rx;
    let r_y = ydh / ry;
    let d = cos2 + sin2 * (r_x * r_x + r_y * r_y);
    return 1.0 / (3.14159265359 * rx * ry * d * d);
}

// The rim colour: the diffuse colour's direction, lifted toward white.
fn hair_rim_colour(mat_dif: vec3<f32>) -> vec3<f32> {
    return normalize(mat_dif) * 0.6 + 0.4;
}

struct HairLit {
    diffuse: vec3<f32>,
    specular: vec3<f32>,
}

// One light on hair. `colour` is the light's colour times its attenuation
// and shadow (the original's `lCol·lDir.w`), `v` the direction to the eye,
// `t`/`b` the strand frame around the shaded normal `n`, `rough` the
// roughness (the original's `Gloss`), `ndv` = |v·n|.
fn light_hair(
    n: vec3<f32>, v: vec3<f32>, t: vec3<f32>, b: vec3<f32>, l: vec3<f32>, colour: vec3<f32>,
    rough: f32, ndv: f32, mat_dif: vec3<f32>, mat_spc: vec3<f32>, rim: vec3<f32>,
) -> HairLit {
    var out: HairLit;
    let ndl = max(dot(n, l), 0.0);
    let k = (rough + 1.0) * (rough + 1.0) * 0.125;
    let d = 0.25 / (ndv * (1.0 - k) + k);
    let a2 = rough * rough;
    let lhv = normalize(l + v);
    let hdv = max(dot(v, lhv), 0.0);
    let e = (-5.55473 * hdv - 6.98316) * hdv;
    let fs0 = 0.05 + 0.4 * exp2(e);
    let fd = 1.0 - fs0;
    let tdh = dot(lhv, t);
    let bdh = dot(lhv, b);
    let ndh = dot(n, lhv);
    let lobe = aniso_d2(ndh, 0.8, a2 * 0.4, tdh, bdh + 0.05) * 0.1;
    let tinted = aniso_d2(ndh, 0.8, a2 * 0.6, tdh, bdh - 0.05) * 0.3;
    let gh = ndl / (ndl * (1.0 - k) + k);
    let fs = fs0 * colour * (gh * d * 3.14159265359);
    out.specular = fs * lobe + fs * tinted * mat_spc;
    let fdc = fd * colour;
    var w = 1.15 - abs(dot(v, lhv));
    w = w * w * max(ndl + 0.75, 0.0);
    out.diffuse = fdc * ndl * mat_dif * 0.4 + fdc * rim * w * w;
    return out;
}

// The ambient's weight on hair: 1.5 − 0.75·(v·n), brighter toward the
// outline (the original's `1.5 + vwr·nrm·0.75`, vwr pointing at the surface).
fn hair_ambient_weight(v: vec3<f32>, n: vec3<f32>) -> f32 {
    return 1.5 - dot(v, n) * 0.75;
}

// The reflection's direction: the normal bent 70 % toward the plane across
// the strands, then mirrored about it.
fn hair_reflection_dir(v: vec3<f32>, n: vec3<f32>, b: vec3<f32>) -> vec3<f32> {
    let vwr = -v;
    let across = cross(vwr, b);
    let r = cross(across, b);
    let m = normalize(mix(n, r, 0.7));
    return reflect(vwr, m);
}

// The reflection's Fresnel weight: 0.1 + 0.5·2^((−5.55473·ndv − 6.98316)·ndv).
fn hair_reflection_weight(ndv: f32) -> f32 {
    return 0.1 + 0.5 * exp2((-5.55473 * ndv - 6.98316) * ndv);
}
