//! Map accounts and the spawn (SPEC_V21 §15.2, §15.5): MAP_SEAL's M2–M7 over a blob, spawn_ok / spawn_v3 and the
//! bounded SlotHashes lookup.
use crate::*;

pub const ENERGY_START: i64 = 39322;
pub const MAX_OBJECTS: usize = 1024;
pub const MAX_ENTRIES: usize = 4096;
pub const MAX_CELL: usize = 64;

impl Map<'_> {
    pub fn n(&self) -> usize { self.n }
}

/// bodymodel.spawn_ok: W none, H = 0, inside the arena disc, and clear of every perch by r + 2Q.
pub fn spawn_ok(wm: &Map, x: i64, y: i64) -> bool {
    let lim = ARENA_R - EDGE_MARGIN;
    if wm.w(x, y).is_some() || wm.h(x, y) != 0 || x * x + y * y > lim * lim { return false; }
    (0..wm.n).all(|i| {
        let o = wm.obj(i);
        let rr = o.r + 2 * Q;
        !o.perch() || (x - o.x) * (x - o.x) + (y - o.y) * (y - o.y) >= rr * rr
    })
}

/// spawn_v3(seed): the first of eight candidates passing spawn_ok, else the origin.
pub fn spawn_v3(wm: &Map, seed: &[u8; 32]) -> State {
    for i in 0..8 {
        let angle = u16::from_le_bytes([seed[4 * i], seed[4 * i + 1]]) as i64;
        let r = (20 + (u16::from_le_bytes([seed[4 * i + 2], seed[4 * i + 3]]) as i64) % 40) * Q;
        let (x, y) = ((r * cos_q(angle)) >> 16, (r * sin_q(angle)) >> 16);
        if spawn_ok(wm, x, y) { return State { x, y, heading: (angle + 32768) & 0xFFFF, ..Default::default() }; }
    }
    State::default()
}

/// SlotHashes data: u64 LE count (≤ 512), then count × (slot u64 LE, hash 32). Err on a bad count or short data.
pub fn slot_hash(d: &[u8], slot: u64) -> Result<Option<[u8; 32]>, ()> {
    if d.len() < 8 { return Err(()); }
    let cnt = u64::from_le_bytes(d[..8].try_into().unwrap());
    if cnt > 512 || (d.len() as u64) < 8 + 40 * cnt { return Err(()); }
    for k in 0..cnt as usize {
        let o = 8 + 40 * k;
        if u64::from_le_bytes(d[o..o + 8].try_into().unwrap()) == slot { return Ok(Some(d[o + 8..o + 40].try_into().unwrap())); }
    }
    Ok(None)
}

/// MAP_SEAL's M2–M7 over a blob: M2's checked parse first, then one linear pass (objects, cells, M7).
pub fn seal_check(b: &[u8]) -> bool {
    if b.len() < 16 || &b[..4] != b"DRZM" { return false; }
    let (n, nw) = (rd16(b, 4) as usize, rd16(b, 6) as usize);
    let (cx0, cy0) = (rd16(b, 8) as i16 as i64, rd16(b, 10) as i16 as i64);
    let (gw, gh) = (rd16(b, 12) as usize, rd16(b, 14) as usize);
    if n == 0 || n > MAX_OBJECTS || nw > n || gw == 0 || gh == 0 { return false; }
    let cells = gw as u64 * gh as u64;
    let g = 16 + 17 * n + 2 * nw;
    let e0 = g as u64 + 2 * (cells + 1);
    if (b.len() as u64) < e0 { return false; }
    let e0 = e0 as usize;
    let total = rd16(b, e0 - 2) as usize;
    if total > MAX_ENTRIES || b.len() != e0 + 2 * total { return false; }
    let m = Map { b, n, nw, cx0, cy0, gw: gw as i64, gh: gh as i64 };
    let (mut bx0, mut by0, mut bx1, mut by1) = (i64::MAX, i64::MAX, i64::MIN, i64::MIN);
    let (mut area, mut k) = (0u64, 0usize);
    for i in 0..n {
        let o = m.obj(i);
        if o.x.abs() > 1 << 24 || o.y.abs() > 1 << 24 || o.r < 1 || o.r > 1 << 22 { return false; }
        let code = b[16 + 16 * n + i];
        if o.h == -1 {
            if code != 0 || k >= nw || rd16(b, g - 2 * nw + 2 * k) as usize != i { return false; }
            k += 1;
        } else if o.h < 0 || o.h > 1 << 22 || code > 7 { return false; }
        let (lx, hx, ly, hy) = ((o.x - o.r) >> CELL_SHIFT, (o.x + o.r) >> CELL_SHIFT, (o.y - o.r) >> CELL_SHIFT, (o.y + o.r) >> CELL_SHIFT);
        bx0 = bx0.min(lx); bx1 = bx1.max(hx); by0 = by0.min(ly); by1 = by1.max(hy);
        area += ((hx - lx + 1) * (hy - ly + 1)) as u64;
    }
    if k != nw || bx0 != cx0 || by0 != cy0 || bx1 - bx0 + 1 != gw as i64 || by1 - by0 + 1 != gh as i64 || area != total as u64 || rd16(b, g) != 0 {
        return false;
    }
    for gy in 0..gh {
        let cy = gy as i64 + cy0;
        for gx in 0..gw {
            let cx = gx as i64 + cx0;
            let c = g + 2 * (gy * gw + gx);
            let (s, e) = (rd16(b, c) as usize, rd16(b, c + 2) as usize);
            if e < s || e > total || e - s > MAX_CELL { return false; }
            let mut last = -1i64;
            for q in s..e {
                let i = rd16(b, e0 + 2 * q) as i64;
                if i <= last || i >= n as i64 { return false; }
                last = i;
                let o = m.obj(i as usize);
                if (o.x - o.r) >> CELL_SHIFT > cx || (o.x + o.r) >> CELL_SHIFT < cx || (o.y - o.r) >> CELL_SHIFT > cy || (o.y + o.r) >> CELL_SHIFT < cy {
                    return false;
                }
            }
        }
    }
    spawn_ok(&m, 0, 0)
}
