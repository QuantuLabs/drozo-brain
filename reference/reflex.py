"""Run each stimulus once on a brain at rest and print what fires.

Needs only this repository and numpy:  python3 reference/reflex.py
"""
import csv, hashlib, json, shutil, sys, tempfile
from pathlib import Path
import numpy as np

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "reference"))
import core_ref

core = ROOT / "data" / "core_v1"
meta = json.loads((core / "model.json").read_text())
sign = np.repeat(np.fromfile(core / "src_sign.i8", dtype="i1").astype(np.int64),
                 np.diff(np.fromfile(core / "row_offsets.u32", dtype="<u4").astype(np.int64)))
v = sign * np.fromfile(core / "edge_syn.u16", dtype="<u2").astype(np.int64) * meta["w_syn_q16"] * meta["gain_q16"]
delta = (np.sign(v) * ((np.abs(v) + (1 << 15)) >> 16)).astype("<i4").tobytes()
assert hashlib.sha256(delta).hexdigest() == meta["array_sha256"]["edge_delta.i32"], "weights do not match model.json"

with tempfile.TemporaryDirectory() as tmp:
    shutil.copytree(core, tmp, dirs_exist_ok=True)
    (Path(tmp) / "edge_delta.i32").write_bytes(delta)
    model = core_ref.CoreModel(tmp)

rows = list(csv.DictReader(open(ROOT / "data" / "neurons.csv")))
watch = {"giant fibre": [i for i, r in enumerate(rows) if r["type"] == "DNp01"],
         "TTMn": [i for i, r in enumerate(rows) if r["type"] == "TTMn"]}
for kind, name in enumerate(("sugar", "water", "bitter", "touch", "looming"), 1):
    spikes, _ = core_ref.run_epoch(model, core_ref.zero_state(model), kind)
    first = {}
    for tick, ids in enumerate(spikes):
        for i in ids:
            first.setdefault(int(i), tick)
    seen = ", ".join(f"{label} first spike at step {[first.get(i, 'never') for i in ids]}" for label, ids in watch.items())
    print(f"{name:8} {sum(len(s) for s in spikes):5} spikes from {len(first):4} neurons | {seen}")
