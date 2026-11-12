//! Maps and the body on-chain (§15): MAP_WRITE, MAP_SEAL, the sealed-map predicate, BODY with the first body's spawn,
//! and the BODY / SPAWN records. The kinematics are the `drozo_body` crate (the spike's bit-exact port).

use super::*;
use drozo_body::{map, Map, State, STATE_BYTES};

pub const OP_MAP_WRITE: u8 = 13;
pub const OP_MAP_SEAL: u8 = 14;
pub const OP_BODY: u8 = 15;
const REC_BODY: u8 = 6;
const REC_SPAWN: u8 = 7;
const MAP_HDR: usize = 48;
const NONE16: u16 = 0xFFFF;
const SLOT_HASHES: [u8; 32] = [
    6, 167, 213, 23, 25, 47, 10, 175, 198, 242, 101, 227, 251, 119, 204, 122, 218, 130, 197, 41, 208, 190, 59, 19, 110,
    45, 0, 85, 32, 0, 0, 0,
];

// Body block (§15.3), after the identity trailer.
pub const B_MAP: usize = O_TR + 184;
const B_EPOCH: usize = B_MAP + 32;
const B_TICK: usize = B_EPOCH + 2;
const B_FLYING: usize = B_TICK + 2;
const B_ENERGY: usize = B_FLYING + 2;
const B_POSE: usize = B_ENERGY + 4;
const B_OBJECT: usize = B_POSE + 1;
const B_STATE: usize = B_OBJECT + 2;
const _: () = assert!(B_MAP == 30_478 && B_STATE + STATE_BYTES == V5_LEN);

#[inline(always)]
fn body_pending() -> ProgramError {
    ProgramError::Custom(4)
}
#[inline(always)]
fn not_ready() -> ProgramError {
    ProgramError::Custom(5)
}

/// Lengths of the other program-owned account types (model, construction, fly); a map is any other length > 48.
#[inline(always)]
fn map_len(n: usize) -> bool {
    n > MAP_HDR && n != STEP && n != 2 * STEP && n != V5_LEN && n != M_TOTAL
}

/// The sealed-map predicate (§15.2), with the read-only role.
pub fn sealed_map(program_id: &Address, m: &AccountView) -> ProgramResult {
    let d = m.try_borrow()?;
    if !m.owned_by(program_id)
        || m.is_writable()
        || !map_len(d.len())
        || &d[0..4] != b"DVMP"
        || u16_at(&d, 4) != 1
        || u16_at(&d, 6) != 0
        || u32_at(&d, 8) as usize != d.len() - MAP_HDR
        || u32_at(&d, 12) != 0
    {
        return Err(invalid());
    }
    Ok(())
}

/// MAP_WRITE [13, offset u32, bytes] and MAP_SEAL [14] (map ws), shapes checked by the dispatcher.
pub fn map_op(program_id: &Address, m: &AccountView, ix: &[u8]) -> ProgramResult {
    if !m.is_signer() {
        return Err(ProgramError::MissingRequiredSignature);
    }
    let mut d = m.try_borrow_mut()?;
    if !m.owned_by(program_id) || !m.is_writable() || !map_len(d.len()) || !zero(&d[..MAP_HDR]) {
        return Err(invalid());
    }
    if ix[0] == OP_MAP_WRITE {
        let off = MAP_HDR as u64 + u32_at(ix, 1) as u64;
        let bytes = &ix[5..];
        if off + bytes.len() as u64 > d.len() as u64 {
            return Err(bad_ix());
        }
        d[off as usize..off as usize + bytes.len()].copy_from_slice(bytes);
        return Ok(());
    }
    let l = d.len() - MAP_HDR;
    if !map::seal_check(&d[MAP_HDR..]) {
        return Err(invalid());
    }
    let mut h = [0u8; 32];
    let s = [d.as_ptr() as u64 + MAP_HDR as u64, l as u64];
    unsafe { pinocchio::syscalls::sol_sha256(s.as_ptr() as *const u8, 1, h.as_mut_ptr()) };
    d[0..4].copy_from_slice(b"DVMP");
    d[4..6].copy_from_slice(&1u16.to_le_bytes());
    put_u32(&mut d, 8, l as u32);
    d[16..48].copy_from_slice(&h);
    Ok(())
}

/// Body block of a fly just finished by INIT: the bound map, no body yet.
pub fn init_block(d: &mut [u8], map_key: &[u8]) {
    d[B_MAP..B_EPOCH].copy_from_slice(map_key);
    d[B_EPOCH..B_TICK].copy_from_slice(&NONE16.to_le_bytes());
    d[B_OBJECT..B_STATE].copy_from_slice(&NONE16.to_le_bytes());
}

/// NEXT_ACTION's precondition: the sealed epoch's body is complete.
#[inline(always)]
pub fn bodied(d: &[u8], sealed: u16) -> ProgramResult {
    if u16_at(d, B_EPOCH) != sealed {
        return Err(body_pending());
    }
    Ok(())
}

/// Record header without ctl_seq (BODY, SPAWN): "drz1" | 3 | type | fly | birth_slot | epoch.
fn rec(b: &mut [u8], ty: u8, fly: &Address, d: &[u8], epoch: u16) {
    b[0..4].copy_from_slice(b"drz1");
    b[4] = REC_V3;
    b[5] = ty;
    b[6..38].copy_from_slice(fly.as_array());
    b[38..46].copy_from_slice(&d[T5_BIRTH..T5_NAME_LEN]);
    b[46..48].copy_from_slice(&epoch.to_le_bytes());
}

/// BODY record from the stored block: state, energy, pose, object.
fn body_record(d: &[u8], fly: &Address, epoch: u16) {
    let mut b = [0u8; 103];
    rec(&mut b, REC_BODY, fly, d, epoch);
    b[48..96].copy_from_slice(&d[B_STATE..V5_LEN]);
    b[96..103].copy_from_slice(&d[B_ENERGY..B_STATE]);
    log_slice(&b);
}

/// BODY [15, epoch u16, until u16] (map ro, fly w, SlotHashes ro): checks 1–10 of §15.6 in order, then the ticks
/// [body_tick, until) of the sealed epoch; at 400 the end state, pose, object, energy and the BODY record.
pub fn body_op(program_id: &Address, accounts: &[AccountView], ix: &[u8]) -> ProgramResult {
    let (m, fly, sh) = (&accounts[0], &accounts[1], &accounts[2]);
    let until = u16_at(ix, 3);
    if until == 0 || until as u32 > TICKS {
        return Err(bad_ix());
    }
    if !fly.owned_by(program_id) || !fly.is_writable() {
        return Err(invalid());
    }
    let mut d = fly.try_borrow_mut()?;
    check_header(&d)?;
    if *m.address().as_array() != key(&d, B_MAP) || *sh.address().as_array() != SLOT_HASHES || sh.is_writable() {
        return Err(invalid());
    }
    sealed_map(program_id, m)?;
    let md = m.try_borrow()?;
    let blob = &md[MAP_HDR..];
    let epoch = u16_at(ix, 1);
    if epoch != u16_at(&d, H_EPOCH) {
        return Err(bad_ix());
    }
    if u16_at(&d, T5_SEALED) != epoch {
        return Err(not_ready());
    }
    let (be, tick, flying, energy) = (u16_at(&d, B_EPOCH), u16_at(&d, B_TICK), u16_at(&d, B_FLYING), u32_at(&d, B_ENERGY));
    let object = u16_at(&d, B_OBJECT);
    let fresh = be == NONE16 && tick == 0;
    if d[H_PHASE] != PHASE_IDLE
        || u32_at(&d, H_TICK) != (epoch as u32 + 1) * TICKS
        || (be != NONE16 && be > epoch)
        || tick as u32 >= TICKS
        || (tick > 0 && be.wrapping_add(1) != epoch)
        || flying > tick
        || energy > 65_536
        || d[B_POSE] > 8
        || (object != NONE16 && object >= u16_at(blob, 4))
        || (fresh && (energy != 0 || flying != 0 || d[B_POSE] != 0 || object != NONE16 || !zero(&d[B_STATE..V5_LEN])))
    {
        return Err(invalid());
    }
    if be == epoch {
        body_record(&d, fly.address(), epoch);
        return Ok(());
    }
    if be.wrapping_add(1) != epoch {
        return Err(invalid());
    }
    if until <= tick {
        return Err(bad_ix());
    }
    let wm = Map::new(blob).ok_or_else(invalid)?;
    let (mut s, mut flying, e0) = if fresh {
        let birth = u64_at(&d, T5_BIRTH);
        if slot()? <= birth {
            return Err(not_ready());
        }
        let found = map::slot_hash(&sh.try_borrow()?, birth).map_err(|_| invalid())?;
        let mut b = [0u8; 129];
        rec(&mut b, REC_SPAWN, fly.address(), &d, 0);
        let s = match found {
            Some(h) => {
                let mut seed = [0u8; 32];
                let p = [
                    b"drozo.spawn.v3".as_ptr() as u64, 14,
                    m.address().as_array().as_ptr() as u64, 32,
                    fly.address().as_array().as_ptr() as u64, 32,
                    d.as_ptr() as u64 + T5_BIRTH as u64, 8,
                    h.as_ptr() as u64, 32,
                ];
                unsafe { pinocchio::syscalls::sol_sha256(p.as_ptr() as *const u8, 5, seed.as_mut_ptr()) };
                b[48] = 1;
                b[49..81].copy_from_slice(&h);
                map::spawn_v3(&wm, &seed)
            }
            None => State::default(),
        };
        s.pack((&mut b[81..129]).try_into().unwrap());
        log_slice(&b);
        (s, 0u32, map::ENERGY_START)
    } else {
        let s = State::unpack(&d[B_STATE..V5_LEN]);
        (s, flying as u32, energy as i64)
    };
    if tick == 0 {
        s.recoiled = 0;
        flying = 0;
    }
    let (action, kind) = (d[H_ACTION], d[H_KIND]);
    let g = drozo_body::gate(action);
    {
        let c = &d[O_COUNTS..O_TR];
        for t in tick as usize..until as usize {
            let n = [0, 1, 2, 3, 4, 5].map(|p| count(c, p, t) as i64);
            drozo_body::step(&mut s, t as i64, n, g, kind, &wm);
            flying += (s.mode == 1) as u32;
        }
    }
    let mut st = [0u8; STATE_BYTES];
    s.pack(&mut st);
    d[B_STATE..V5_LEN].copy_from_slice(&st);
    if (until as u32) < TICKS {
        d[B_TICK..B_FLYING].copy_from_slice(&until.to_le_bytes());
        d[B_FLYING..B_ENERGY].copy_from_slice(&(flying as u16).to_le_bytes());
        put_u32(&mut d, B_ENERGY, e0 as u32);
        return Ok(());
    }
    let (pose, obj) = wm.pose_kind(&s);
    let e = drozo_body::next_energy(e0, action, flying);
    d[B_EPOCH..B_TICK].copy_from_slice(&epoch.to_le_bytes());
    d[B_TICK..B_ENERGY].fill(0);
    put_u32(&mut d, B_ENERGY, e as u32);
    d[B_POSE] = pose;
    d[B_OBJECT..B_STATE].copy_from_slice(&obj.unwrap_or(NONE16).to_le_bytes());
    body_record(&d, fly.address(), epoch);
    Ok(())
}
