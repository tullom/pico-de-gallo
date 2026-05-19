# Installing the Toolchain

To use *Pico de Gallo* from a host PC, you need the `gallo`
command-line tool. That's it — `gallo` will speak to the firmware
over USB and you don't need any extra drivers on most platforms.

There are two ways to get it: pre-built binaries (fastest), or
building from source.

## Option A — Pre-built Binaries

Pre-built binaries are attached to every `application-v*` release
on the
[Releases](https://github.com/OpenDevicePartnership/pico-de-gallo/releases)
page. Supported triples:

| OS      | Architectures      |
|---------|--------------------|
| Linux   | `x86_64`, `aarch64`|
| Windows | `x86_64`, `aarch64`|
| macOS   | `aarch64`          |

Download the right archive for your system, unzip, and put `gallo`
(or `gallo.exe`) somewhere on your `PATH`.

```console
$ gallo --version
gallo 0.8.0
```

## Option B — Build from Source

If your platform isn't in the table above, or you want to live on
`main`:

1. Install [Rust](https://rustup.rs/) (stable toolchain, 1.90 or
   newer — the workspace pins MSRV to 1.90).
2. Clone the repo:
   ```console
   $ git clone https://github.com/OpenDevicePartnership/pico-de-gallo
   $ cd pico-de-gallo/crates
   ```
3. Build the CLI:
   ```console
   $ cargo build --release -p gallo
   ```
4. The binary lives at `target/release/gallo` (or `gallo.exe` on
   Windows). Move or symlink it into a directory on your `PATH`.

> [!TIP]
>
> On Linux you may want to install the `libudev` headers first so
> `nusb` builds without extra steps:
>
> ```console
> $ sudo apt install libudev-dev pkg-config
> ```

## Optional Extras

You only need these if you're working **on** Pico de Gallo, not
just **with** it:

- **The mdBook source** for this book lives under `book/`. Build
  with `mdbook build book`.
- **The C FFI library** (`pico-de-gallo-ffi`) builds a `.so` /
  `.dylib` / `.dll` shared library plus a generated `pico_de_gallo.h`
  header. See [`crates/ffi.md`](../crates/ffi.md).
- **The Python bindings** (`pyco-de-gallo`) build with
  [maturin](https://www.maturin.rs/):
  ```console
  $ pip install maturin
  $ cd crates/pyco-de-gallo
  $ maturin develop --release
  ```
  See [`crates/python.md`](../crates/python.md).

## Next

Now [verify your device](./verify.md) is talking to the host.
