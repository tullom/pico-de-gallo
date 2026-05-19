# Publishing the driver

Once the driver feels good locally, spend ten extra minutes making it a
crate other people will trust.

## Cargo metadata

Start with the basics in `Cargo.toml`:

```toml
[package]
name = "tmp102"
version = "0.1.0"
edition = "2024"
description = "embedded-hal driver for the TMP102 I2C temperature sensor"
license = "MIT OR Apache-2.0"
repository = "https://github.com/OpenDevicePartnership/pico-de-gallo"
categories = ["embedded", "hardware-support", "no-std"]
keywords = ["tmp102", "temperature", "sensor", "i2c", "embedded-hal"]
```

If the crate is `no_std`, say so clearly in the README and crate docs.
If async support is optional, document the feature flag right next to the
first example.

## docs.rs

If your docs need specific features enabled, tell docs.rs explicitly:

```toml
[package.metadata.docs.rs]
all-features = false
features = ["async"]
rustdoc-args = ["--cfg", "docsrs"]
```

That avoids the common "works locally, missing items on docs.rs" trap.

## README essentials

At minimum, include:

- what the device is
- which traits the crate implements or expects
- a blocking example
- an async example if you support one
- feature flags
- wiring or address-selection notes for `A0`

For this driver, also mention that `pico-de-gallo-hal` is only used for
examples and tests; it should stay in `[dev-dependencies]` so downstream
users do not pay for it.

## Optional `defmt` support

If you want the crate to fit nicely into embedded firmware projects,
optional `defmt` support is a nice touch:

- gate it behind a feature
- make the dependency optional
- keep the default feature set small

## Release hygiene

Two last bits of boring professionalism matter a lot:

- keep a `CHANGELOG.md`
- follow semver when you change the public API

Small drivers live a long time. A clean README, useful crate metadata,
and predictable releases do more for adoption than one more clever type.
