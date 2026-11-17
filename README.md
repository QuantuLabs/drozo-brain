# Drozo brain

The brain of Drozo: 2,839 neurons of a real fruit fly, wired exactly as its connectome records them, running entirely on Solana.

Every spike is computed by a Solana program, in integers. No weight is trained and no server thinks for the fly. Anyone can rebuild the program to the same bytes and replay a brain to the same state.

It is written as a [SolTensor](https://github.com/QuantuLabs/soltensor) model: the generic SolTensor compiler turns this description into the on-chain program.

| | |
|---|---|
| Neurons | 2,839, of 1,183 cell types |
| Connections | 204,153: every connection the connectome records between these neurons |
| Synapses carried by the on-chain weights | 2,533,893 |
| Synapses in the source data for these connections | 2,588,357 |
| Sensory inputs | 500 neurons, 5 senses × 100 |
| Motor outputs | 72 neurons in 6 readout groups |
| Trained parameters | 0 |
| Brain steps per stimulus | 400 |

## The wiring is the connectome's

The circuit comes from MaleCNS v1.0, the map of the full central nervous system of a male *Drosophila melanogaster*: 165,122 neurons.

The 2,839 neurons span the brain and the nerve cord: 969 brain interneurons, 744 nerve-cord interneurons, 403 sensory neurons, 242 ascending and 155 descending neurons that link brain and cord, 157 motor neurons, 109 visual projection neurons and 60 others. `data/neurons.csv` gives the MaleCNS body id, cell type and class of each one, so any neuron can be looked up in the dataset.

Between these neurons, the model holds every connection MaleCNS records and no other, each with its synapse count. `reference/wiring.py` compares the two:

```
$ python3 reference/wiring.py connectome-weights-male-cns-v1.0-minconf-0.5.feather
model: 204,153 connections, 2,588,357 synapses
MaleCNS, between the same 2,839 neurons: 204,153 connections, 2,588,357 synapses
IDENTICAL
```

No weight is learned. A connection's weight is its real synapse count, signed by the source neuron and scaled by two global constants:

```
edge_delta = R16(src_sign * edge_syn * w_syn_q16 * gain_q16)
R16(v) = v / 2^16, rounded half away from zero
```

Each connection's weight therefore gives its synapse count back exactly. 192,983 connections have a non-zero
weight, for 2,533,893 synapses. The other 11,170 connections leave 203 neurons whose sign is 0: their weight is 0, so
their 54,464 synapses have no effect and cannot be read back from the chain.

## The senses and the muscles are named neurons

A stimulus drives 100 sensory neurons, chosen by their official MaleCNS annotation. The 500 input neurons are all different.

| Stimulus | The 100 neurons it drives | MaleCNS annotation |
|---|---|---|
| Touch | tactile neurons of the body bristles | class `mechanosensory_tactile` |
| Looming shadow | LPLC2 visual neurons, 52 right and 48 left | type `LPLC2` |
| Sugar | taste neurons of the legs, the labellum and the pharynx | class `gustatory`, cholinergic |
| Water | taste neurons of wing and leg bristles | class `gustatory`, receptor `putative_IR52b` |
| Bitter | taste neurons of leg and wing bristles | class `gustatory`, receptor `putative_ppk23` |

The brain answers through 72 motor and efferent neurons, the ones the inputs reach through the wiring. The game reads them in six groups: accept food, reject food, forward, left, right, startle.

## A known reflex comes out of the wiring

LPLC2 neurons detect an approaching object and excite the giant fibres, which trigger the escape jump (Klapoetke et al., *Nature* 2017, doi:10.1038/nature24626). That pathway is in the model: the LPLC2 neurons reach the two giant fibres through 100 connections (2,795 synapses), and the giant fibres reach the two TTMn, the motor neurons of the jump muscle, through 2 connections (90 synapses).

Start a brain at rest and play each stimulus once:

```
$ python3 reference/reflex.py
sugar     1395 spikes from  750 neurons | giant fibre first spike at step ['never', 'never'], TTMn first spike at step ['never', 'never']
water      471 spikes from  218 neurons | giant fibre first spike at step ['never', 'never'], TTMn first spike at step ['never', 'never']
bitter     345 spikes from  198 neurons | giant fibre first spike at step ['never', 'never'], TTMn first spike at step ['never', 'never']
touch     1445 spikes from  676 neurons | giant fibre first spike at step ['never', 'never'], TTMn first spike at step ['never', 'never']
looming   2189 spikes from  892 neurons | giant fibre first spike at step [40, 40], TTMn first spike at step [72, 102]
```

The looming shadow fires both giant fibres, then both jump motor neurons. The four other stimuli fire neither. No weight was set for this: the two global constants scale all 204,153 connections alike.

## Every spike is computed on-chain

- Neurons are integer leaky integrate-and-fire units. One stimulus is 400 brain steps, with a synaptic delay of 4 steps and a refractory period of 5.
- Each fly has its own brain: the state of all its neurons lives in that fly's account, and each transaction advances it by a bounded number of steps.
- The arithmetic is exact. Rounding, reset and refractory rules are fixed, so a replay gives the same bytes on any machine. `reference/core_ref.py` is the reference; `soltensor test` runs it on 5 recorded cases and checks the final state bytes and the readout counts.
- The build is reproducible: `soltensor build` gives the program Drozo runs, byte for byte.

## How the 2,839 were selected

1. The full connectome runs as an integer leaky integrate-and-fire network (the *teacher*).
2. The teacher was simulated on 36,000 stimuli (636 journeys of five kinds: sugar, water, bitter, touch, looming
   shadow), on top of a 115-event training corpus. Every neuron that fired at least once was kept: 2,839 neurons,
   1.7 % of the connectome. In the last 15,000 stimuli a single new neuron appeared.
3. The core runs with the teacher's arithmetic and weights, unchanged. A neuron that never fires sends nothing, so
   removing it changes no spike. On the five recorded cases in `tests/fixtures`, the core was identical to the full
   teacher at every step.

## Limits

- The selection is empirical, not a proof: a stimulus sequence outside the explored set could make a removed neuron
  fire.
- MaleCNS has no sugar, water or bitter label. The three taste groups are real taste neurons; which group plays which
  taste is a modelling choice. The receptor labels in the table are the dataset's own.
- A stimulus is a fixed drive on its 100 neurons during the first 100 steps. There is no receptor or eye model, and no
  intensity.
- The names of the six output groups are how the game reads real motor neurons, not labels from the dataset.

## Build

With [SolTensor](https://github.com/QuantuLabs/soltensor) installed (its README lists the Solana
tools: `cargo-build-sbf` 4.1.0 and platform-tools v1.54). SolTensor is released separately, after this
repository:

```bash
soltensor analyze      # sizes and host checks
soltensor build        # target/drozo.so and target/drozo.model.bin
soltensor test         # host reference on the 5 recorded cases
shasum -a 256 target/drozo.so
# 1f5abfaa937845f566962eb797f46fb874c959cf57bcf2b6888b494005547862  target/drozo.so
```

Two profiles share the model and the shell; they differ only by `shell/fee_keys.<profile>.json` and the pin:
- devnet (default): `soltensor build` → `target/drozo.so`, sha256 `1f5abfaa…7862`, 118,216 bytes.
- mainnet: `soltensor build mainnet/drozo_mainnet.py` → `target/drozo_mainnet.so`, sha256 `0fdf09ab5d0bc8a5fb39c3a96fe6c7f4e74054f64f9b595464185f2ac64947be`, 118,216 bytes.

`build` produces the program Drozo runs on devnet, byte for byte (118,216 bytes, SBPF v0), and
refuses anything else. It needs no wallet, validator or network. The whole inference runs on-chain:
SolTensor only prepares the model image, the certificate tables, the constants and the Rust brain
code. Deploying the program and driving flies belongs to the Drozo app. When a fly is created, the program takes the creation fee itself and sends it to the treasury.

The identical binary was checked from several folders and with a moved cargo home, on macOS arm64
only.

## What is here

| Path | Content |
|---|---|
| `drozo.py` | the model description read by SolTensor |
| `mainnet/drozo_mainnet.py` | the same model with the mainnet profile and pin |
| `data/core_v1/` | the circuit: neurons, signed connections, sources, drives, readouts |
| `data/neurons.csv` | the 2,839 neurons: MaleCNS body id, cell type, class, and the input or output group each belongs to |
| `data/calibration.json` | compute-unit weights of the program's steps |
| `shell/` | the hand-written Rust around the brain: accounts, fees, body, records; `build_pins.json` makes the build give the same bytes from any folder |
| `reference/core_ref.py` | the reference simulator the circuit was validated with |
| `reference/reflex.py` | plays each stimulus on a brain at rest and prints what fires |
| `reference/wiring.py` | compares the model's connections with the MaleCNS file |
| `tests/fixtures/` | 5 recorded cases: state before, stimulus, state after |

## Data and license

The code is released under the [Apache-2.0](LICENSE) license. Copyright 2026 QuantuLabs; see [NOTICE](NOTICE).
The circuit is derived from the MaleCNS v1.0 connectome (Janelia FlyEM); see [DATA.md](DATA.md) for the credit
and terms.
