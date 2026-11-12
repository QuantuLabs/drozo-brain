from soltensor import spiking as sn

model = sn.network(
    core=sn.core("data/core_v1/model.json"),
    neuron=sn.lif_q30_v21(),  # exact rounding, reset and refractory rules
    readout=sn.readout(
        thresholds=(1, 1, 1, 1, 1, 1, 0, 0),
        weights_q16=(65536, 65536, 16384, 65536, 65536, 0, 0, 0),
        actions={1: (0, 4), 2: (0, 4), 3: (1, 5), 4: (5, 6), 5: (5, 6)},
        motions=((2, 1), (3, 2), (4, 3)),
        select="lowest_threshold_then_order"),
    catch_up=sn.lazy_certificate(bucket_bits=12, linear_slack=185),
    units=sn.calibration("data/calibration.json"),
    shell=sn.rust_shell("shell", fee_profile="shell/fee_keys.devnet.json"),
    expect=sn.exact(
        arch="v0",
        image_sha256="824490e29a1d006909d58434b69ae07a8d5afa76600327de6658c224c626566d",
        elf_sha256="a98f6b207946e39b54972a5228590f50ccc911a070d09e411a8e1c53367df65d"),
)
