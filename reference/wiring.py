"""Check that the model holds every connection MaleCNS v1.0 records between its neurons, and no other.

Needs numpy, pyarrow and the connectome file from Janelia (see DATA.md):
  python3 reference/wiring.py connectome-weights-male-cns-v1.0-minconf-0.5.feather
"""
import csv, sys
from pathlib import Path
import numpy as np
import pyarrow.feather as feather

ROOT = Path(__file__).resolve().parent.parent
core = ROOT / "data" / "core_v1"
body = np.array([int(r["body_id"]) for r in csv.DictReader(open(ROOT / "data" / "neurons.csv"))], dtype=np.int64)
offsets = np.fromfile(core / "row_offsets.u32", dtype="<u4").astype(np.int64)
src = np.repeat(np.arange(body.size), np.diff(offsets))
dst = np.fromfile(core / "edge_dst.u16", dtype="<u2").astype(np.int64)
syn = np.fromfile(core / "edge_syn.u16", dtype="<u2").astype(np.int64)
model = set(zip(body[src].tolist(), body[dst].tolist(), syn.tolist()))

table = feather.read_table(sys.argv[1], memory_map=True)
pre, post, weight = (table.column(n).to_numpy() for n in ("body_pre", "body_post", "weight"))
keep = np.flatnonzero(np.isin(pre, body))
keep = keep[np.isin(post[keep], body)]
source = set(zip(pre[keep].tolist(), post[keep].tolist(), weight[keep].tolist()))

print(f"model: {len(model):,} connections, {int(syn.sum()):,} synapses")
print(f"MaleCNS, between the same {body.size:,} neurons: {len(source):,} connections, {int(weight[keep].sum()):,} synapses")
print("IDENTICAL" if model == source else f"DIFFERENT: {len(model - source)} only in the model, {len(source - model)} only in MaleCNS")
sys.exit(model != source)
