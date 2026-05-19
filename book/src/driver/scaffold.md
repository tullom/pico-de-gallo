# Scaffolding the crate

We are going to build an ordinary Rust library crate. No board support
package, no cross toolchain, no custom linker setup.

Start with a fresh library:

```console
$ cargo new --lib tmp102
    Creating library `tmp102` package
$ cd tmp102
```

Now add the dependencies we know we will need:

```console
$ cargo add embedded-hal
$ cargo add embedded-hal-async
$ cargo add device-driver --no-default-features -F toml
$ cargo add --dev pico-de-gallo-hal --git https://github.com/OpenDevicePartnership/pico-de-gallo
$ cargo add --dev tokio -F rt-multi-thread,time,macros
```

That gives us three layers:

- `embedded-hal` for the blocking driver surface
- `embedded-hal-async` for the async sibling
- `device-driver` for generated register accessors

And in `dev-dependencies`:

- `pico-de-gallo-hal` so tests and examples can talk to real hardware
- `tokio` for async examples and hardware-in-the-loop tests

> [!TIP]
> Keep `pico-de-gallo-hal` in `[dev-dependencies]`, not in normal
> `[dependencies]`. Your end users should depend on your driver crate,
> not on the host-side test harness you used while writing it.

Your `Cargo.toml` should look roughly like this:

```toml
[package]
name = "tmp102"
version = "0.1.0"
edition = "2024"

[dependencies]
device-driver = { version = "1.0.7", default-features = false, features = ["toml"] }
embedded-hal = "1.0.0"
embedded-hal-async = "1.0.0"

[dev-dependencies]
pico-de-gallo-hal = { git = "https://github.com/OpenDevicePartnership/pico-de-gallo" }
tokio = { version = "1.47.1", features = ["rt-multi-thread", "time", "macros"] }
```

We also want the code generator that turns a register description into a
Rust interface:

```console
$ cargo install device-driver-cli
```

That binary reads a TOML description of the device and emits the boring
part of the driver for us: register wrappers, field accessors, and the
plumbing around them.

At this point the crate is still empty, but we have already set the
shape of the project:

- library-first
- `embedded-hal`-based
- async-friendly
- hardware-testable on the host

Next we describe TMP102's registers in a form the generator can digest.
