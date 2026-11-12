"""Standalone exact reference for the Drozo exact core (numpy only, no repo import).

Semantics are the teacher's (examples/drozo/drozomodel/reference.py) restricted to the core neurons:
per tick, slot = tick % 4; due = pending[slot]; pending[slot] = 0;
g_total = g + due + stim (i32 checked); if refractory: ref -= 1, x unchanged, g = g_total;
else x_next = R30(A*x) + R30(C*g_total), g_next = R30(B*g_total) (i32 checked);
spike iff not refractory and x_next > T -> x = 0, g = 0, ref = 5;
spikes of tick t deliver edge_delta into pending[slot] (consumed at tick t + 4).
R30(v) = round half away from zero of v / 2**30.
"""
import json
from pathlib import Path
import numpy as np

class CoreModel:
    def __init__(self, directory):
        d = Path(directory)
        self.meta = json.loads((d / "model.json").read_text())
        k = self.meta["neurons"]
        self.K = k
        self.D = self.meta["delay_steps"]
        self.A = np.int64(self.meta["a_q30"]); self.B = np.int64(self.meta["b_q30"])
        self.C = np.int64(self.meta["c_q30"]); self.T = np.int64(self.meta["threshold_q16"])
        self.REF = int(self.meta["refractory_steps"]); self.TICKS = int(self.meta["epoch_ticks"])
        self.row_offsets = np.fromfile(d / "row_offsets.u32", dtype="<u4").astype(np.int64)
        self.edge_dst = np.fromfile(d / "edge_dst.u16", dtype="<u2").astype(np.int64)
        self.edge_delta = np.fromfile(d / "edge_delta.i32", dtype="<i4").astype(np.int64)
        assert self.row_offsets.size == k + 1 and self.row_offsets[-1] == self.edge_dst.size == self.edge_delta.size
        self.sources = {int(kind): np.asarray(ids, dtype=np.int64) for kind, ids in self.meta["sources"].items()}
        self.drives = {int(kind): tuple(v) for kind, v in self.meta["drives"].items()}
        self.readouts = [np.asarray(ids, dtype=np.int64) for ids in self.meta["readouts"]]

def r30(v):
    v = np.asarray(v, dtype=np.int64)
    mag = (np.abs(v) + (np.int64(1) << np.int64(29))) >> np.int64(30)
    return np.where(v < 0, -mag, mag)

def zero_state(model):
    k, d = model.K, model.D
    return {"x": np.zeros(k, np.int32), "g": np.zeros(k, np.int32), "ref": np.zeros(k, np.uint8),
            "pending": np.zeros((d, k), np.int64)}

def _check_i32(a, what):
    if a.size and (a.max() > 2**31 - 1 or a.min() < -2**31):
        raise OverflowError(what)

def step(model, st, tick, kind):
    """Advance one tick (tick = 0..399 inside the epoch). Returns spiking local ids (ascending)."""
    k = model.K
    stim = np.zeros(k, dtype=np.int64)
    start, ticks, dq = model.drives.get(kind, (0, 0, 0))
    if kind and dq and start <= tick < start + ticks:
        stim[model.sources[kind]] = dq
    slot = tick % model.D
    due = st["pending"][slot].copy(); st["pending"][slot][:] = 0
    g_total = st["g"].astype(np.int64) + due + stim
    _check_i32(g_total, "g_total")
    in_ref = st["ref"] > 0
    x_next = r30(model.A * st["x"].astype(np.int64)) + r30(model.C * g_total)
    g_next = r30(model.B * g_total)
    _check_i32(x_next, "x_next"); _check_i32(g_next, "g_next")
    spike = (~in_ref) & (x_next > model.T)
    st["x"][:] = np.where(spike, 0, np.where(in_ref, st["x"], x_next)).astype(np.int32)
    st["g"][:] = np.where(spike, 0, np.where(in_ref, g_total, g_next)).astype(np.int32)
    st["ref"][:] = np.where(spike, model.REF, np.where(in_ref, st["ref"] - 1, st["ref"])).astype(np.uint8)
    rows = np.flatnonzero(spike)
    if rows.size:
        cnt = model.row_offsets[rows + 1] - model.row_offsets[rows]
        tot = int(cnt.sum())
        if tot:
            idx = np.repeat(model.row_offsets[rows], cnt) + np.arange(tot) - np.repeat(np.cumsum(cnt) - cnt, cnt)
            add = np.zeros(k, dtype=np.int64)
            np.add.at(add, model.edge_dst[idx], model.edge_delta[idx])
            st["pending"][slot] += add
    return rows

def run_epoch(model, st, kind):
    spikes = [step(model, st, tick, kind) for tick in range(model.TICKS)]
    counts = np.zeros(model.K, dtype=np.int64)
    for rows in spikes:
        counts[rows] += 1
    readouts = [int(counts[r].sum()) for r in model.readouts]
    return spikes, readouts

def state_bytes(st):
    """Canonical little-endian image: x i32[K] | g i32[K] | ref u8[K] | pending i64[4][K] (slot-major)."""
    return (st["x"].astype("<i4").tobytes() + st["g"].astype("<i4").tobytes()
            + st["ref"].astype("u1").tobytes() + st["pending"].astype("<i8").tobytes())

def load_state(path, model):
    raw = Path(path).read_bytes(); k, d = model.K, model.D
    x = np.frombuffer(raw, "<i4", k, 0).astype(np.int32).copy()
    g = np.frombuffer(raw, "<i4", k, 4 * k).astype(np.int32).copy()
    ref = np.frombuffer(raw, "u1", k, 8 * k).astype(np.uint8).copy()
    pending = np.frombuffer(raw, "<i8", d * k, 9 * k).reshape(d, k).astype(np.int64).copy()
    return {"x": x, "g": g, "ref": ref, "pending": pending}
