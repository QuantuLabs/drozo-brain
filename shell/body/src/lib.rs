//! Integer port of examples/drozo/world/bodymodel.py (end state, flying ticks, pose kind, energy); bit-exact.
#![no_std]

const Q: i64 = 65536;
pub const TICKS: usize = 400;
const W_FWD: i64 = 16384;
const W_LR: i64 = 65536;
const TURN_IMPULSE: i64 = 182;
const FWD_IMPULSE: i64 = 12288;
const ARENA_R: i64 = 150 * Q;
const EDGE_MARGIN: i64 = 10 * Q;
const EDGE_TURN: i64 = 256;
const OMEGA_DEADBAND: i64 = 8;
const V_DEADBAND: i64 = 32;
const SI_DEADBAND: i64 = 16;
const WT_DEADBAND: i64 = 16;
const ENERGY_FEED: i64 = 16384;
const ENERGY_METABOLISM: i64 = 1966;
const ENERGY_FLY_TICK: i64 = 13;
pub const ACTION_FEED_ACCEPT: u8 = 4;
const ACTION_FEED_REJECT: u8 = 5;
const ACTION_STARTLE: u8 = 6;
const WALK: i64 = 0;
const FLY: i64 = 1;
const NONE: u8 = 0;
const FEED: u8 = 1;
const REJECT: u8 = 2;
const ESCAPE: u8 = 3;
const RESERVE: i64 = 100;
const DESC_DZ: i64 = 4800;
const MIN_FLY: i64 = 20;
const FLY_DESCEND_AT: i64 = 60;
const STARTLE_CUTOFF: i64 = TICKS as i64 - RESERVE - 70;
const FLY_HEIGHT: i64 = 4 * Q;
const FLY_CLEAR: i64 = Q;
const FLY_V: i64 = 9830;
const DIVERT_V: i64 = 58982;
const FLY_DV: i64 = 1311;
const DZ_MAX: i64 = 5000;
const LAND_EPS: i64 = 3277;
const OMEGA_FLY_MAX: i64 = 546;
const ATTRACT_R: i64 = 6 * Q;
const HOP_T: i64 = 24;
const HOP_H: i64 = 39322;
const HOP_PUSH: i64 = 3277;
const HOP_DZ: i64 = 2293;
const FLINCH_T: i64 = 12;
const FEED_T: i64 = 60;
const REJECT_T: i64 = 30;
const RECOIL_T: i64 = 20;
const RECOIL_V: i64 = -3277;
const CLIMB_DZ: i64 = 3277;
const DROP_DZ: i64 = 7864;
const CELL_SHIFT: u32 = 19; // CELL = 8 * Q

/// SIN_Q16[0..=256] (quarter wave); the full table is its odd/even reflection.
const SINQ: [i32; 257] = [
    0, 402, 804, 1206, 1608, 2010, 2412, 2814, 3216, 3617, 4019, 4420, 4821, 5222, 5623, 6023,
    6424, 6824, 7224, 7623, 8022, 8421, 8820, 9218, 9616, 10014, 10411, 10808, 11204, 11600, 11996, 12391,
    12785, 13180, 13573, 13966, 14359, 14751, 15143, 15534, 15924, 16314, 16703, 17091, 17479, 17867, 18253, 18639,
    19024, 19409, 19792, 20175, 20557, 20939, 21320, 21699, 22078, 22457, 22834, 23210, 23586, 23961, 24335, 24708,
    25080, 25451, 25821, 26190, 26558, 26925, 27291, 27656, 28020, 28383, 28745, 29106, 29466, 29824, 30182, 30538,
    30893, 31248, 31600, 31952, 32303, 32652, 33000, 33347, 33692, 34037, 34380, 34721, 35062, 35401, 35738, 36075,
    36410, 36744, 37076, 37407, 37736, 38064, 38391, 38716, 39040, 39362, 39683, 40002, 40320, 40636, 40951, 41264,
    41576, 41886, 42194, 42501, 42806, 43110, 43412, 43713, 44011, 44308, 44604, 44898, 45190, 45480, 45769, 46056,
    46341, 46624, 46906, 47186, 47464, 47741, 48015, 48288, 48559, 48828, 49095, 49361, 49624, 49886, 50146, 50404,
    50660, 50914, 51166, 51417, 51665, 51911, 52156, 52398, 52639, 52878, 53114, 53349, 53581, 53812, 54040, 54267,
    54491, 54714, 54934, 55152, 55368, 55582, 55794, 56004, 56212, 56418, 56621, 56823, 57022, 57219, 57414, 57607,
    57798, 57986, 58172, 58356, 58538, 58718, 58896, 59071, 59244, 59415, 59583, 59750, 59914, 60075, 60235, 60392,
    60547, 60700, 60851, 60999, 61145, 61288, 61429, 61568, 61705, 61839, 61971, 62101, 62228, 62353, 62476, 62596,
    62714, 62830, 62943, 63054, 63162, 63268, 63372, 63473, 63572, 63668, 63763, 63854, 63944, 64031, 64115, 64197,
    64277, 64354, 64429, 64501, 64571, 64639, 64704, 64766, 64827, 64884, 64940, 64993, 65043, 65091, 65137, 65180,
    65220, 65259, 65294, 65328, 65358, 65387, 65413, 65436, 65457, 65476, 65492, 65505, 65516, 65525, 65531, 65535,
    65536,
];

#[inline]
fn sin_i(i: i64) -> i64 {
    let i = (i & 1023) as usize;
    let (q, k) = (i >> 8, i & 255);
    match q {
        0 => SINQ[k] as i64,
        1 => SINQ[256 - k] as i64,
        2 => -(SINQ[k] as i64),
        _ => -(SINQ[256 - k] as i64),
    }
}
#[inline]
pub fn sin_q(h: i64) -> i64 { sin_i(h >> 6) }
#[inline]
pub fn cos_q(h: i64) -> i64 { sin_i((h >> 6) + 256) }

#[inline]
fn sign(v: i64) -> i64 { (v > 0) as i64 - (v < 0) as i64 }
#[inline]
fn decay(x: i64, shift: u32, dead: i64) -> i64 {
    let x = x - sign(x) * (x.abs() >> shift);
    if x.abs() < dead { 0 } else { x }
}
#[inline]
fn ease(x: i64, target: i64, shift: u32, dead: i64) -> i64 {
    let d = target - x;
    let x = x + sign(d) * (d.abs() >> shift);
    if x.abs() < dead && target == 0 { 0 } else { x }
}
#[inline]
fn clamp(v: i64, lo: i64, hi: i64) -> i64 { if v < lo { lo } else if v > hi { hi } else { v } }
/// Floor division for b > 0, on SBF's unsigned div64 (signed i64 division is a slow software routine there).
#[inline]
fn fdiv(a: i64, b: i64) -> i64 {
    if a >= 0 { (a as u64 / b as u64) as i64 } else { -(((a.unsigned_abs() + b as u64 - 1) / b as u64) as i64) }
}
#[inline]
fn cdiv(a: i64, b: i64) -> i64 { -fdiv(-a, b) }

/// Binary angle of (dx, dy), 256 directions; first maximum like Python max() over h = 0, 256, ...
/// The dot product is unimodal in h, so a 32-direction coarse pass then a +-8 window finds the same argmax.
pub fn iatan2(dy: i64, dx: i64) -> i64 {
    let dot = |k: i64| dx * cos_q(k << 8) + dy * sin_q(k << 8);
    let (mut c, mut cv) = (0, i64::MIN);
    let mut k = 0;
    while k < 256 {
        let v = dot(k);
        if v > cv { cv = v; c = k; }
        k += 8;
    }
    let (mut best, mut bv) = (c, cv);
    for d in -8..=8 {
        let k = (c + d) & 255;
        let v = dot(k);
        if v > bv || (v == bv && k < best) { bv = v; best = k; }
    }
    best << 8
}

fn isqrt(n: u64) -> u64 {
    if n < 2 { return n; }
    let mut x = 1u64 << ((64 - n.leading_zeros()) / 2 + 1);
    let mut y = (x + n / x) / 2;
    while y < x { x = y; y = (x + n / x) / 2; }
    x
}

fn arena(x: i64, y: i64) -> (i64, i64) {
    let d2 = x * x + y * y;
    if d2 <= ARENA_R * ARENA_R { return (x, y); }
    let mut d = isqrt(d2 as u64) as i64;
    d += (d * d < d2) as i64;
    let d = d as u64;
    (sign(x) * (x.unsigned_abs() * ARENA_R as u64 / d) as i64, sign(y) * (y.unsigned_abs() * ARENA_R as u64 / d) as i64)
}

#[inline]
fn arc(hop_t: i64) -> i64 { HOP_H * sin_q(fdiv((HOP_T - hop_t) * 32768, HOP_T - 1)) >> 16 }
#[inline]
fn land_budget(z: i64) -> i64 { cdiv(z, DESC_DZ) + RESERVE }
#[inline]
fn settle(z: i64, h: i64, hop: bool) -> i64 {
    let rate = if hop { HOP_DZ } else if h > z { CLIMB_DZ } else { DROP_DZ };
    cdiv((h - z).abs(), rate)
}

/// Read-only view of map.bin (see gen_map.py). Object handles are indices.
#[derive(Clone, Copy)]
pub struct Map<'a> {
    b: &'a [u8],
    n: usize,
    nw: usize,
    cx0: i64,
    cy0: i64,
    gw: i64,
    gh: i64,
}

#[inline]
fn rd16(b: &[u8], o: usize) -> u16 { u16::from_le_bytes([b[o], b[o + 1]]) }
#[inline]
fn rd32(b: &[u8], o: usize) -> i32 { i32::from_le_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]]) }

#[derive(Clone, Copy)]
pub struct Obj { pub x: i64, pub y: i64, pub r: i64, pub h: i64, pub i: u16 }
impl Obj {
    #[inline]
    fn perch(&self) -> bool { self.h >= 0 }
}

impl<'a> Map<'a> {
    pub fn new(b: &'a [u8]) -> Option<Self> {
        if b.len() < 16 || &b[..4] != b"DRZM" { return None; }
        let n = rd16(b, 4) as usize;
        let nw = rd16(b, 6) as usize;
        let (gw, gh) = (rd16(b, 12) as i64, rd16(b, 14) as i64);
        let m = Map { b, n, nw, cx0: rd16(b, 8) as i16 as i64, cy0: rd16(b, 10) as i16 as i64, gw, gh };
        let cells = (gw * gh) as usize;
        let end = m.grid() + 2 * (cells + 1);
        if b.len() < end || b.len() != end + 2 * rd16(b, end - 2) as usize { return None; }
        Some(m)
    }
    #[inline]
    fn grid(&self) -> usize { 16 + 17 * self.n + 2 * self.nw }
    #[inline]
    pub fn obj(&self, i: usize) -> Obj {
        let o: &[u8; 16] = self.b[16 + 16 * i..32 + 16 * i].try_into().unwrap();
        let f = |k: usize| i32::from_le_bytes([o[k], o[k + 1], o[k + 2], o[k + 3]]) as i64;
        Obj { x: f(0), y: f(4), r: f(8), h: f(12), i: i as u16 }
    }
    #[inline]
    fn pose_code(&self, i: usize) -> u8 { self.b[16 + 16 * self.n + i] }
    /// Object indices of grid cell (cx, cy).
    #[inline]
    fn cell(&self, cx: i64, cy: i64) -> core::ops::Range<usize> {
        let (gx, gy) = (cx - self.cx0, cy - self.cy0);
        if gx < 0 || gy < 0 || gx >= self.gw || gy >= self.gh { return 0..0; }
        let g = self.grid() + 2 * (gy * self.gw + gx) as usize;
        let base = self.grid() + 2 * ((self.gw * self.gh) as usize + 1);
        base + 2 * rd16(self.b, g) as usize..base + 2 * rd16(self.b, g + 2) as usize
    }
    #[inline]
    fn at(&self, x: i64, y: i64, mut f: impl FnMut(Obj) -> bool) {
        for c in self.b[self.cell(x >> CELL_SHIFT, y >> CELL_SHIFT)].chunks_exact(2) {
            let o = self.obj(u16::from_le_bytes([c[0], c[1]]) as usize);
            if (x - o.x) * (x - o.x) + (y - o.y) * (y - o.y) <= o.r * o.r && !f(o) { return; }
        }
    }
    pub fn h(&self, x: i64, y: i64) -> i64 {
        let mut m = 0;
        self.at(x, y, |o| { if o.perch() && o.h > m { m = o.h; } true });
        m
    }
    pub fn w(&self, x: i64, y: i64) -> Option<Obj> {
        let mut r = None;
        self.at(x, y, |o| if o.perch() { true } else { r = Some(o); false });
        r
    }
    pub fn perch_id(&self, x: i64, y: i64, z: i64) -> Option<u16> {
        let mut r = None;
        self.at(x, y, |o| if o.perch() && o.h == z { r = Some(o.i); false } else { true });
        r
    }
    pub fn blocker(&self, x: i64, y: i64, z: i64) -> Option<Obj> {
        let mut r: Option<Obj> = None;
        self.at(x, y, |o| {
            if o.perch() && o.h > z && r.map_or(true, |b| o.h > b.h) { r = Some(o); }
            true
        });
        r
    }
    pub fn nearest_perch(&self, x: i64, y: i64, within: i64) -> Option<Obj> {
        let mut best: Option<(i64, Obj)> = None;
        let (cx, cy) = (x >> CELL_SHIFT, y >> CELL_SHIFT);
        for gx in cx - 1..cx + 2 {
            for gy in cy - 1..cy + 2 {
                for c in self.b[self.cell(gx, gy)].chunks_exact(2) {
                    let o = self.obj(u16::from_le_bytes([c[0], c[1]]) as usize);
                    let d2 = (x - o.x) * (x - o.x) + (y - o.y) * (y - o.y);
                    if o.perch() && d2 <= within * within && best.map_or(true, |b| d2 < b.0) { best = Some((d2, o)); }
                }
            }
        }
        best.map(|b| b.1)
    }
    /// (pose code, object index) of a landed state; codes: gen_map.py POSE.
    pub fn pose_kind(&self, s: &State) -> (u8, Option<u16>) {
        let (x, y, z) = (s.x as i64, s.y as i64, s.z as i64);
        if z > 0 {
            if let Some(i) = self.perch_id(x, y, z) { return (self.pose_code(i as usize), Some(i)); }
        }
        if z == 0 && self.w(x, y).is_none() {
            for k in 0..self.nw {
                let o = self.obj(rd16(self.b, 16 + 17 * self.n + 2 * k) as usize);
                if (x - o.x) * (x - o.x) + (y - o.y) * (y - o.y) <= (o.r + Q) * (o.r + Q) { return (8, Some(o.i)); }
            }
        }
        (0, None)
    }
}

/// Persisted integer body state (field order of bodymodel.origin()).
#[derive(Clone, Copy, Default, PartialEq, Eq, Debug)]
pub struct State {
    pub x: i64, pub y: i64, pub z: i64, pub heading: i64, pub v: i64, pub omega: i64, pub vz: i64, pub mode: i64,
    pub fly_t: i64, pub feed_t: i64, pub reject_t: i64, pub wt: i64, pub vt: i64, pub si: i64, pub flinch_t: i64,
    pub hop_t: i64, pub recoil_t: i64, pub recoiled: i64, pub lift_t: i64,
    /// CU per step section (feature "prof", SBF only): [0..9] sections, 9 = probe overhead, 10 = last reading.
    #[cfg(feature = "prof")]
    pub prof: [u64; 11],
}

pub const STATE_BYTES: usize = 48;

#[cfg(feature = "prof")]
extern "C" {
    fn sol_remaining_compute_units() -> u64;
}

#[inline(always)]
fn mark(_s: &mut State, _k: usize) {
    #[cfg(feature = "prof")]
    {
        let now = unsafe { sol_remaining_compute_units() };
        if _s.prof[10] != 0 { _s.prof[_k] += _s.prof[10] - now; }
        _s.prof[10] = unsafe { sol_remaining_compute_units() };
    }
}

impl State {
    /// 9 x i32 (x y z v omega vz wt vt sI), u16 heading, u16 fly_t, 8 x u8 (mode feed_t reject_t flinch_t hop_t
    /// recoil_t recoiled lift_t); every field's range fits (timers <= 170, heading u16).
    pub fn pack(&self, o: &mut [u8; STATE_BYTES]) {
        let w = [self.x, self.y, self.z, self.v, self.omega, self.vz, self.wt, self.vt, self.si];
        for (k, v) in w.iter().enumerate() { o[4 * k..4 * k + 4].copy_from_slice(&(*v as i32).to_le_bytes()); }
        o[36..38].copy_from_slice(&(self.heading as u16).to_le_bytes());
        o[38..40].copy_from_slice(&(self.fly_t as u16).to_le_bytes());
        let b = [self.mode, self.feed_t, self.reject_t, self.flinch_t, self.hop_t, self.recoil_t, self.recoiled, self.lift_t];
        for (k, v) in b.iter().enumerate() { o[40 + k] = *v as u8; }
    }
    pub fn unpack(o: &[u8]) -> State {
        let i = |k: usize| rd32(o, 4 * k) as i64;
        State {
            x: i(0), y: i(1), z: i(2), v: i(3), omega: i(4), vz: i(5), wt: i(6), vt: i(7), si: i(8),
            heading: rd16(o, 36) as i64, fly_t: rd16(o, 38) as i64, mode: o[40] as i64, feed_t: o[41] as i64,
            reject_t: o[42] as i64, flinch_t: o[43] as i64, hop_t: o[44] as i64, recoil_t: o[45] as i64,
            recoiled: o[46] as i64, lift_t: o[47] as i64,
            ..Default::default()
        }
    }
}

pub fn gate(action: u8) -> u8 {
    match action { ACTION_STARTLE => ESCAPE, ACTION_FEED_ACCEPT => FEED, ACTION_FEED_REJECT => REJECT, _ => NONE }
}

fn steer(s: &mut State, want: i64) {
    let d = ((want - s.heading + 32768) & 0xFFFF) - 32768;
    s.omega = clamp(d, -OMEGA_FLY_MAX, OMEGA_FLY_MAX);
}

/// One tick of bodymodel.step (flags and the rendered z dropped: they do not feed the state).
/// c = (accept, reject, forward, left, right, startle) counts at tick t.
pub fn step(s: &mut State, t: i64, c: [i64; 6], g: u8, kind: u8, wm: &Map) {
    mark(s, 9);
    let [a, j, f, l, r, st] = c;
    s.si = decay(s.si, 4, SI_DEADBAND) + st * 256;
    let walking = s.mode == WALK;
    let left = TICKS as i64 - 1 - t;
    if g == FEED && a > 0 { s.feed_t = FEED_T; }
    if j > 0 {
        s.reject_t = REJECT_T;
        if g == REJECT && s.recoiled == 0 { s.recoiled = 1; s.recoil_t = RECOIL_T; s.v = RECOIL_V; s.vt = 0; }
    }
    if st > 0 && g != ESCAPE { s.flinch_t = FLINCH_T; }
    if g == ESCAPE && walking && t < STARTLE_CUTOFF {
        let z0 = s.z + if s.hop_t != 0 { arc(s.hop_t + 1) } else { 0 };
        let air = left - land_budget(FLY_HEIGHT.max(z0 + FLY_CLEAR));
        let thr = if kind == 5 { 2 * 256 } else { 3 * 256 };
        if s.si >= thr && air >= FLY_DESCEND_AT + MIN_FLY {
            s.z = z0; s.mode = FLY; s.hop_t = 0; s.vz = 0;
            s.fly_t = (if kind == 5 { 170 } else { 110 }).min(air);
            s.lift_t = MIN_FLY;
        } else if st > 0 && s.hop_t == 0 && HOP_T - 1 <= left && settle(s.z, wm.h(s.x, s.y), true) <= left {
            s.hop_t = HOP_T;
            s.vt += HOP_PUSH;
        }
    }
    let flying = s.mode == FLY;
    mark(s, 0);
    s.wt = decay(s.wt + ((l - r) * TURN_IMPULSE * W_LR >> 16), 4, WT_DEADBAND);
    s.omega = ease(s.omega, s.wt, 3, OMEGA_DEADBAND);
    if flying {
        s.vt = 0;
    } else {
        s.vt = decay(s.vt + (f * FWD_IMPULSE * W_FWD >> 16), 4, V_DEADBAND);
        s.v = ease(s.v, s.vt, 3, V_DEADBAND);
        if s.feed_t > 0 {
            s.v -= sign(s.v) * (s.v.abs() >> 2);
            s.omega -= sign(s.omega) * (s.omega.abs() >> 2);
            s.vt -= sign(s.vt) * (s.vt.abs() >> 2);
            s.wt -= sign(s.wt) * (s.wt.abs() >> 2);
        }
    }
    let (x, y, h) = (s.x, s.y, s.heading);
    mark(s, 1);
    if flying {
        let v0 = s.v;
        if left <= land_budget(s.z) + 1 { s.fly_t = 0; }
        let descent = s.fly_t <= FLY_DESCEND_AT;
        let water = if descent { wm.w(x, y) } else { None };
        let ground = wm.h(x, y);
        let zt;
        if let Some(w) = water {
            steer(s, iatan2(y - w.y, x - w.x));
            s.v = ease(s.v, DIVERT_V, 2, 0);
            zt = if s.z > ground { s.z } else { ground + FLY_CLEAR };
        } else if descent {
            if let Some(p) = wm.nearest_perch(x, y, ATTRACT_R) {
                if (x - p.x) * (x - p.x) + (y - p.y) * (y - p.y) > (p.r >> 1) * (p.r >> 1) { steer(s, iatan2(p.y - y, p.x - x)); }
            }
            s.v = decay(s.v, 3, V_DEADBAND);
            zt = ground;
        } else {
            let ahead = wm.h(x + 4 * cos_q(h), y + 4 * sin_q(h));
            zt = FLY_HEIGHT.max(ground.max(ahead) + FLY_CLEAR).max(s.z + if s.lift_t != 0 { Q / 2 } else { 0 });
            s.v = FLY_V;
        }
        s.v = v0 + clamp(s.v - v0, -FLY_DV, FLY_DV);
        s.vz = ease(s.vz, clamp((zt - s.z) >> 2, -DZ_MAX, DZ_MAX), 1, 0);
        s.vz = clamp(s.vz, -DZ_MAX, DZ_MAX);
        s.z += s.vz;
        s.fly_t = (s.fly_t - 1).max(0);
        s.lift_t = (s.lift_t - 1).max(0);
    }
    mark(s, 2);
    if x * x + y * y > (ARENA_R - EDGE_MARGIN) * (ARENA_R - EDGE_MARGIN) {
        s.omega += sign(sin_q(h) * x - cos_q(h) * y) * EDGE_TURN;
    }
    let mut tan = None;
    if flying {
        s.omega = clamp(s.omega, -OMEGA_FLY_MAX, OMEGA_FLY_MAX);
        let hp = (h + s.omega) & 0xFFFF;
        let (bx, by) = arena(x + (s.v * cos_q(hp) >> 16), y + (s.v * sin_q(hp) >> 16));
        if let Some(o) = wm.blocker(bx, by, s.z) {
            let a = iatan2(y - o.y, x - o.x);
            let (c1, c2) = ((a + 16384) & 0xFFFF, (a - 16384) & 0xFFFF);
            let k = |c: i64| (((c - h + 32768) & 0xFFFF) - 32768).abs();
            let tc = if k(c2) < k(c1) { c2 } else { c1 };
            tan = Some(tc);
            steer(s, tc);
        }
    }
    mark(s, 3);
    s.heading = (h + s.omega) & 0xFFFF;
    let h = s.heading;
    let (mut nx, mut ny) = arena(x + (s.v * cos_q(h) >> 16), y + (s.v * sin_q(h) >> 16));
    mark(s, 4);
    if flying {
        let mut hn = wm.h(nx, ny);
        if let Some(tc) = tan {
            if hn > s.z {
                (nx, ny) = arena(x + (s.v * cos_q(tc) >> 16), y + (s.v * sin_q(tc) >> 16));
                hn = wm.h(nx, ny);
            }
        }
        if hn <= s.z { s.x = nx; s.y = ny; }
        let ground = if hn <= s.z { hn } else { wm.h(s.x, s.y) };
        let dry_near = (s.z - ground).abs() <= DZ_MAX && wm.w(s.x, s.y).is_none();
        let normal = s.fly_t == 0 && (s.z - ground).abs() <= LAND_EPS && (s.z - s.vz - ground).abs() <= DZ_MAX;
        if (dry_near && normal) || t == TICKS as i64 - 1 {
            s.mode = WALK; s.z = ground; s.vz = 0; s.fly_t = 0; s.lift_t = 0;
        }
    } else {
        let (x0, y0, z0) = (x, y, s.z);
        let water = wm.w(nx, ny);
        let (hn, hc) = (wm.h(nx, ny), wm.h(x, y));
        let climb = if s.hop_t != 0 { HOP_DZ } else { CLIMB_DZ };
        let d2 = |w: &Obj, px: i64, py: i64| (px - w.x) * (px - w.x) + (py - w.y) * (py - w.y);
        let blocked = match water {
            Some(w) => !(wm.w(x, y).map_or(false, |cw| cw.i == w.i) && d2(&w, nx, ny) > d2(&w, x, y)),
            None => false,
        };
        if blocked {
            let w = water.unwrap();
            let (wx, wy) = (w.x - x, w.y - y);
            let sg = sign(sin_q(h) * wx - cos_q(h) * wy);
            s.omega += (if sg == 0 { 1 } else { sg }) * EDGE_TURN;
        } else if hn > s.z && hn > hc {
            s.z = hn.min(s.z + climb);
            s.v -= sign(s.v) * (s.v.abs() >> 2);
        } else {
            s.x = nx; s.y = ny;
            if hn > s.z { s.z = hn.min(s.z + climb); } else if hn < s.z { s.z = hn.max(s.z - if s.hop_t != 0 { HOP_DZ } else { DROP_DZ }); }
        }
        let hs = if s.x == x && s.y == y { hc } else { hn };
        if settle(s.z, hs, s.hop_t > 1) > left {
            s.x = x0; s.y = y0; s.z = z0;
            if hc != z0 {
                let rate = if s.hop_t != 0 { HOP_DZ } else if hc > z0 { CLIMB_DZ } else { DROP_DZ };
                s.z = z0 + clamp(hc - z0, -rate, rate);
            }
        }
    }
    mark(s, if flying { 5 } else { 6 });
    if s.mode == WALK && s.hop_t > 0 { s.hop_t -= 1; }
    s.feed_t = (s.feed_t - 1).max(0);
    s.reject_t = (s.reject_t - 1).max(0);
    s.flinch_t = (s.flinch_t - 1).max(0);
    s.recoil_t = (s.recoil_t - 1).max(0);
    mark(s, 7);
}

pub struct Epoch { pub end: State, pub flying: u32, pub pose: (u8, Option<u16>), pub energy: i64 }

pub mod map;

/// bodymodel.run_epoch end state + pose_kind + next_energy. counts(t) gives the 6 population counts at tick t.
pub fn run_epoch(start: &State, mut counts: impl FnMut(usize) -> [i64; 6], action: u8, kind: u8, energy: i64, wm: &Map) -> Epoch {
    let mut s = State { recoiled: 0, ..*start };
    let g = gate(action);
    let mut flying = 0;
    for t in 0..TICKS {
        let c = counts(t);
        mark(&mut s, 8);
        step(&mut s, t as i64, c, g, kind, wm);
        flying += (s.mode == FLY) as u32;
    }
    let pose = wm.pose_kind(&s);
    Epoch { end: s, flying, pose, energy: next_energy(energy, action, flying) }
}

pub fn next_energy(energy: i64, action: u8, flying: u32) -> i64 {
    let mut e = energy + if action == ACTION_FEED_ACCEPT { ENERGY_FEED } else { 0 };
    e -= ENERGY_METABOLISM + ENERGY_FLY_TICK * flying as i64;
    clamp(e, 0, Q)
}

#[cfg(test)]
mod tests {
    extern crate std;

    fn iatan2_ref(dy: i64, dx: i64) -> i64 {
        let (mut best, mut bv) = (0, i64::MIN);
        let mut h = 0;
        while h < 65536 {
            let v = dx * super::cos_q(h) + dy * super::sin_q(h);
            if v > bv { bv = v; best = h; }
            h += 256;
        }
        best
    }

    #[test]
    fn iatan2_matches_brute_force() {
        let mut r: u64 = 0x8004;
        let mut next = || { r ^= r << 13; r ^= r >> 7; r ^= r << 17; r };
        for i in 0..4_000_000u64 {
            let span = [4, 300, 70_000, 20_000_000][(i & 3) as usize];
            let dx = (next() % (2 * span + 1)) as i64 - span as i64;
            let dy = (next() % (2 * span + 1)) as i64 - span as i64;
            assert_eq!(super::iatan2(dy, dx), iatan2_ref(dy, dx), "dx {dx} dy {dy}");
        }
        for dx in -40..=40 { for dy in -40..=40 { assert_eq!(super::iatan2(dy, dx), iatan2_ref(dy, dx)); } }
    }
}
