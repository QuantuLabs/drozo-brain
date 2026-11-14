from soltensor import spiking as sn

model = sn.variant(
    "../drozo.py",
    shell=sn.rust_shell("../shell", fee_profile="../shell/fee_keys.mainnet.json"),
    expect=sn.exact(
        arch="v0",
        image_sha256="824490e29a1d006909d58434b69ae07a8d5afa76600327de6658c224c626566d",
        elf_sha256="0fdf09ab5d0bc8a5fb39c3a96fe6c7f4e74054f64f9b595464185f2ac64947be"),
)
