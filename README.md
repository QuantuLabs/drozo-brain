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

## Status

This repo will hold the SolTensor version of the model: its definition in the SolTensor format, the data it compiles from, a reference simulator and parity tests. That is the version that will be published.

It lands here once `soltensor build` compiles it into a program that matches the program Drozo runs today, byte for byte.

Private for now.
