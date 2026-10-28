//! Triangle order for the vertex cache, as the original reorders a mesh it
//! converts to render form.
//!
//! Group by group, triangles are drawn greedily: each step takes the
//! unprocessed triangle with the highest score (the sum of its vertices'
//! scores), else the head of the group's list sorted by the starting
//! scores. A vertex scores 2/√n for its n remaining triangles, plus a
//! bonus for its place in a 32-entry cache: 0.75 for the three newest
//! entries, (1 − (k − 3)/29)^1.5 for place k after them. Both curves come
//! from the original's fast logarithm and exponential, reproduced here bit
//! for bit. Meshes flagged so then have each group's triangles sorted by
//! their flags. Vertices are renumbered by first use.

use std::sync::OnceLock;

use crate::mesh_edit::{FaceArrays, FaceGroup, Incidence};

/// Entries of the modelled cache.
const CACHE: usize = 32;
/// The cache buffer: the cache, the new triangle's three vertices.
const CACHE_BUFFER: usize = CACHE + 3;
/// Valences up to this take their score from the table.
const TABLE_VALENCE: i32 = 32;
/// Render-flag bit: sort each group's triangles by their flags (low 28
/// bits) after ordering them.
pub const SORT_BY_FLAGS: u32 = 0x0100_0000;
const FLAG_SORT_MASK: u32 = 0x0FFF_FFFF;

// The fast logarithm: exponent × ln 2 plus a polynomial of the mantissa.
const LN2: u32 = 0x3F31_7218;
const LN_OFFSET: u32 = 0xC2B3_DE55;
const LN_C1: u32 = 0x4059_7523;
const LN_C2: u32 = 0x4011_9122;
const LN_C3: u32 = 0x3CCC_A7FC;
const LN_C4: u32 = 0x3E79_8F62;
// The fast exponential: the scaled argument read as float bits, times a
// polynomial of its mantissa.
const EXP_SCALE: u32 = 0x4B38_AA3B;
const EXP_BIAS: u32 = 0x4E7E_0000;
const EXP_MAX: u32 = 0x4EFF_0000;
const EXP_C0: u32 = 0x3F02_8D05;
const EXP_C1: u32 = 0x3E9F_C08B;
const EXP_C2: u32 = 0x3E2A_9CBE;
const EXP_C3: u32 = 0xBB0B_4858;
const EXP_C4: u32 = 0x3C5D_BE77;

const MANTISSA: u32 = 0x007F_FFFF;
const EXPONENT: u32 = 0x7F80_0000;
const ONE: u32 = 0x3F80_0000;

fn f(bits: u32) -> f32 {
    f32::from_bits(bits)
}

/// The original's fast natural logarithm.
pub fn fast_ln(x: f32) -> f32 {
    let bits = x.to_bits();
    let exponent = (bits >> 23) as i32 as f32 * f(LN2);
    let offset = if x <= 0.0 {
        f32::NEG_INFINITY
    } else {
        f(LN_OFFSET)
    };
    let m = f((bits & MANTISSA) | ONE);
    let m2 = m * m;
    let mut p = m * f(LN_C3) - f(LN_C4);
    p *= m2;
    p += m - f(LN_C2);
    p *= m2;
    p += exponent;
    (m * f(LN_C1) + offset) + p
}

/// The original's fast exponential.
pub fn fast_exp(x: f32) -> f32 {
    let t = x * f(EXP_SCALE) + f(EXP_BIAS);
    let t = if t < f(EXP_MAX) { t } else { f(EXP_MAX) };
    let i = (t.round_ties_even() as i32).max(0) as u32;
    let m = f((i & MANTISSA) | ONE);
    let p = (((m * f(EXP_C4) + f(EXP_C3)) * m + f(EXP_C2)) * m + f(EXP_C1)) * m + f(EXP_C0);
    f(i & EXPONENT) * p
}

struct Tables {
    /// Score of a vertex with n remaining triangles, n ≤ 32.
    valence: [f32; TABLE_VALENCE as usize + 1],
    /// Bonus of each cache place.
    place: [f32; CACHE_BUFFER],
}

fn tables() -> &'static Tables {
    static TABLES: OnceLock<Tables> = OnceLock::new();
    TABLES.get_or_init(|| {
        let mut valence = [0.0; TABLE_VALENCE as usize + 1];
        for (n, v) in valence.iter_mut().enumerate().skip(1) {
            *v = fast_exp(fast_ln(n as f32) * -0.5) * 2.0;
        }
        let mut place = [0.0; CACHE_BUFFER];
        place[..3].fill(0.75);
        for (k, p) in place.iter_mut().enumerate().take(CACHE).skip(3) {
            let d = (k as f64 - 3.0) * 0.034_482_758_620_689_655;
            *p = fast_exp(fast_ln((1.0 - d) as f32) * 1.5);
        }
        Tables { valence, place }
    })
}

/// The score of a vertex with `n` remaining triangles.
pub fn valence_score(n: i32) -> f32 {
    if n <= TABLE_VALENCE {
        tables().valence[n.max(0) as usize]
    } else {
        fast_exp(fast_ln(n as f32) * -0.5) * 2.0
    }
}

/// The bonus of cache place `k`.
pub fn place_score(k: usize) -> f32 {
    tables().place.get(k).copied().unwrap_or(0.0)
}

/// Sorts the linked list from `first` by score, highest first, with a
/// bottom-up merge sort that takes the earlier of equal scores; relinks
/// both directions and returns the new head.
fn sort_list(first: i32, links: &mut [(i32, i32)], score: &[f32]) -> i32 {
    let mut list = first;
    let mut size = 1;
    loop {
        let mut p = list;
        list = -1;
        let mut tail = -1i32;
        let mut merges = 0;
        while p >= 0 {
            merges += 1;
            let mut q = p;
            let mut psize = 0;
            for _ in 0..size {
                psize += 1;
                q = links[q as usize].0;
                if q < 0 {
                    break;
                }
            }
            let mut qsize = size;
            while psize > 0 || (qsize > 0 && q >= 0) {
                let e;
                if psize == 0 {
                    e = q;
                    q = links[q as usize].0;
                    qsize -= 1;
                } else if qsize == 0
                    || q < 0
                    || !matches!(
                        score[p as usize].partial_cmp(&score[q as usize]),
                        Some(std::cmp::Ordering::Less)
                    )
                {
                    e = p;
                    p = links[p as usize].0;
                    psize -= 1;
                } else {
                    e = q;
                    q = links[q as usize].0;
                    qsize -= 1;
                }
                if tail >= 0 {
                    links[tail as usize].0 = e;
                } else {
                    list = e;
                }
                links[e as usize].1 = tail;
                tail = e;
            }
            p = q;
        }
        if tail >= 0 {
            links[tail as usize].0 = -1;
        }
        if merges <= 1 {
            return list;
        }
        size *= 2;
    }
}

/// Sorts `a[lo..=hi]` by `key` with the original's middle-pivot quicksort
/// (not stable: the order of equal keys is the algorithm's).
fn sort_by_key(a: &mut [u32], mut lo: i32, hi: i32, key: &impl Fn(u32) -> u32) {
    loop {
        let mut i = lo;
        let mut j = hi;
        let pivot = key(a[((lo as i64 + hi as i64) / 2) as usize]);
        loop {
            while key(a[i as usize]) < pivot {
                i += 1;
            }
            while pivot < key(a[j as usize]) {
                j -= 1;
            }
            if i <= j {
                a.swap(i as usize, j as usize);
                i += 1;
                j -= 1;
            }
            if i > j {
                break;
            }
        }
        if lo < j {
            sort_by_key(a, lo, j, key);
        }
        lo = i;
        if i >= hi {
            break;
        }
    }
}

/// Reorders the triangles of each group for the vertex cache and permutes
/// every per-face array and the incidence's faces to match; returns each
/// unified vertex's new index (by first use), `u32::MAX` for unused ones.
/// `vertex_corners` gives each unified vertex's corners in the incidence.
pub(crate) fn order(
    faces: &mut FaceArrays<'_>,
    inc: &mut Incidence,
    vertex_corners: &[(u32, u32)],
    groups: &[FaceGroup],
    render_flags: u32,
) -> Vec<u32> {
    let nf = faces.tri.len();
    let vertices = vertex_corners.len();
    if nf < 2 {
        return (0..vertices as u32).collect();
    }
    let tri = faces.tri.clone();
    let mut remaining: Vec<i32> = vertex_corners
        .iter()
        .map(|&(s, e)| e as i32 - s as i32)
        .collect();
    let mut score: Vec<f32> = remaining.iter().map(|&n| valence_score(n)).collect();
    let mut tri_score = vec![0.0f32; nf];
    let mut order = vec![-1i32; nf];
    let mut links = vec![(-1i32, -1i32); nf];
    let mut placed = -1i32;
    let sum = |score: &[f32], t: [u32; 3]| {
        score[t[0] as usize] + score[t[1] as usize] + score[t[2] as usize]
    };

    for group in groups {
        if group.end <= group.first || group.end > nf {
            continue;
        }
        let (first, end) = (group.first, group.end);
        let mut best = -1i32;
        let mut best_score = 0.0f32;
        for face in first..end {
            tri_score[face] = sum(&score, tri[face]);
            if best_score < tri_score[face] {
                best_score = tri_score[face];
                best = face as i32;
            }
        }
        for (face, link) in links.iter_mut().enumerate().take(end).skip(first) {
            *link = (face as i32 + 1, face as i32 - 1);
        }
        links[first].1 = -1;
        links[end - 1].0 = -1;
        let mut head = sort_list(first as i32, &mut links, &tri_score);

        let mut cache = [0i32; CACHE_BUFFER];
        let mut next_cache = [0i32; CACHE_BUFFER];
        let mut kept = -1i32;
        for _ in first..end {
            if best < 0 {
                best = head;
            }
            let b = best as usize;
            let t = tri[b];
            for k in 0..3 {
                next_cache[k] = t[k] as i32;
                remaining[t[k] as usize] -= 1;
            }
            placed += 1;
            order[b] = placed;
            let (n, p) = links[b];
            if p >= 0 {
                links[p as usize].0 = n;
            } else {
                head = n;
            }
            if n >= 0 {
                links[n as usize].1 = p;
            }
            let mut last = 2usize;
            for &v in cache.iter().take((kept + 1) as usize) {
                if !t.contains(&(v as u32)) {
                    last += 1;
                    next_cache[last] = v;
                }
            }
            std::mem::swap(&mut cache, &mut next_cache);
            kept = last.min(CACHE - 1) as i32;

            best = -1;
            best_score = 0.0;
            for (k, &v) in cache.iter().enumerate().take(last + 1) {
                let v = v as usize;
                if remaining[v] <= 0 {
                    continue;
                }
                score[v] =
                    valence_score(remaining[v]) + if k < CACHE { place_score(k) } else { 0.0 };
                let (s, e) = vertex_corners[v];
                for c in &inc.corners[s as usize..e as usize] {
                    let face = c.face as usize;
                    if order[face] < 0 {
                        tri_score[face] = sum(&score, tri[face]);
                        if best_score < tri_score[face] {
                            best_score = tri_score[face];
                            best = face as i32;
                        }
                    }
                }
            }
        }
    }

    if render_flags & SORT_BY_FLAGS != 0
        && let Some(flags) = faces.flags.as_deref()
        && order.iter().all(|&o| o >= 0)
    {
        let mut by_place = vec![0u32; nf];
        for (face, &o) in order.iter().enumerate() {
            by_place[o as usize] = face as u32;
        }
        let key = |face: u32| flags[face as usize] & FLAG_SORT_MASK;
        for group in groups {
            if group.end > group.first && group.end <= nf {
                sort_by_key(
                    &mut by_place,
                    group.first as i32,
                    group.end as i32 - 1,
                    &key,
                );
            }
        }
        for (place, &face) in by_place.iter().enumerate() {
            order[face as usize] = place as i32;
        }
    }

    // Faces outside every group keep their place.
    for (face, o) in order.iter_mut().enumerate() {
        if *o < 0 {
            *o = face as i32;
        }
    }
    let permute = |old_index: usize| order[old_index] as usize;
    let mut new_tri = vec![[0u32; 3]; nf];
    for (face, t) in tri.iter().enumerate() {
        new_tri[permute(face)] = *t;
    }
    for c in &mut inc.corners {
        c.face = order[c.face as usize] as u32;
    }
    if let Some(flags) = faces.flags.as_deref_mut() {
        let old = flags.clone();
        for (face, &v) in old.iter().enumerate() {
            flags[permute(face)] = v;
        }
    }
    if let Some(edges) = faces.edges.as_deref_mut() {
        let old = edges.clone();
        for (face, &v) in old.iter().enumerate() {
            edges[permute(face)] = v;
        }
    }
    if let Some(extra) = faces.extra.as_deref_mut() {
        let old = extra.clone();
        for (face, &v) in old.iter().enumerate() {
            extra[permute(face)] = v;
        }
    }
    if let Some(children) = faces.children.as_deref_mut() {
        for c in children.iter_mut().take(nf >> 2) {
            if let Some(&o) = order.get(*c as usize) {
                *c = o as u32;
            }
        }
    }

    let mut renumber = vec![u32::MAX; vertices];
    let mut next = 0u32;
    for t in &mut new_tri {
        for v in t.iter_mut() {
            let slot = &mut renumber[*v as usize];
            if *slot == u32::MAX {
                *slot = next;
                next += 1;
            }
            *v = *slot;
        }
    }
    *faces.tri = new_tri;
    renumber
}

#[cfg(test)]
#[path = "vertex_cache_test.rs"]
mod tests;
