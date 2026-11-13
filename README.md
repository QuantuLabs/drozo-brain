# Drozo model

The brain of Drozo, as a SolTensor model: a real fruit-fly circuit that the generic SolTensor compiler turns into a fully on-chain Solana program.

| | |
|---|---|
| Neurons | 2,839 |
| Weighted connections | 204,153 |
| Synapses behind them | 2,588,357 |
| Sensory inputs | 500 neurons, 5 senses × 100 |
| Motor outputs | 72 neurons in 6 readout groups |
| Trained parameters | 0 |

## Origin

The circuit comes from the MaleCNS connectome, a map of the full central nervous system of a male *Drosophila melanogaster*: 165,122 neurons.

1. The full connectome runs as an integer leaky integrate-and-fire network (the *teacher*).
2. Only about 2,800 of its neurons ever fire. The core is the union of the training probes and every neuron that fired across a 36,000-event exploration of the teacher.
3. The core runs with the teacher's arithmetic and weights, unchanged. It matched the teacher on 306/306 outputs.

No weight is learned. A connection's weight is its real synapse count, signed by the source neuron and scaled by two global constants:

```
edge_delta = R16(src_sign * edge_syn * w_syn_q16 * gain_q16)
R16(v) = v / 2^16, rounded half away from zero
```

## Build

With [SolTensor](https://github.com/QuantuLabs/soltensor) installed (its README lists the Solana
tools: `cargo-build-sbf` 4.1.0 and platform-tools v1.54):

```bash
soltensor analyze      # sizes and host checks
soltensor build        # target/drozo.so and target/drozo.model.bin
soltensor test         # host reference on the 5 recorded cases
shasum -a 256 target/drozo.so
# 1f5abfaa937845f566962eb797f46fb874c959cf57bcf2b6888b494005547862  target/drozo.so
```

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
| `data/core_v1/` | the circuit: neurons, signed connections, sources, drives, readouts |
| `data/calibration.json` | compute-unit weights of the program's steps |
| `shell/` | the hand-written Rust around the brain: accounts, fees, body, records; `build_pins.json` makes the build give the same bytes from any folder |
| `reference/core_ref.py` | the reference simulator the circuit was validated with |
| `tests/fixtures/` | 5 recorded cases: state before, stimulus, state after |

Private for now.
