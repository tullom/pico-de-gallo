# Describe the device for code generation

The fastest way to lose momentum in driver work is to hand-write the
same register boilerplate for the hundredth time. TMP102 is small, but
it still benefits from code generation.

Create `tmp102.toml` in the crate root.

## 1. Global configuration

The `config` block tells `device-driver-cli` what an address looks like,
how to split names into Rust identifiers, and how to interpret the
register layout that follows.

```toml
[config]
register_address_type = "u8"
default_byte_order = "LE"
name_word_boundaries = ["Hyphen"]
```

## 2. Register declarations

Now declare the four registers from the datasheet.

```toml
[Temperature]
type = "register"
address = 0
size_bits = 16
access = "RO"
description = "Temperature register"

[Configuration]
type = "register"
address = 1
size_bits = 16
access = "RW"
description = "Configuration register"

[Tlow]
type = "register"
address = 2
size_bits = 16
access = "RW"
description = "T-low register"

[Thigh]
type = "register"
address = 3
size_bits = 16
access = "RW"
description = "T-high register"
```

That alone is enough to generate raw register accessors. The real value
comes from teaching the generator about individual fields.

## 3. Configuration bitfields

TMP102's configuration register contains exactly the kind of structure
we want to avoid encoding by hand: enums hidden inside bit ranges.

```toml
[Configuration.fields.SD]
description = "Shutdown mode"
base = "uint"
start = 0
end = 1

[Configuration.fields.SD.conversion]
name = "shutdown-mode"
description = "Shutdown mode"
running = 0
power-off = 1

[Configuration.fields.TM]
description = "Thermostat mode"
base = "uint"
start = 1
end = 2

[Configuration.fields.TM.conversion]
name = "thermostat-mode"
description = "Thermostat mode of operation"
comparator = 0
interrupt = 1

[Configuration.fields.POL]
description = "Alert pin polarity"
base = "uint"
start = 2
end = 3

[Configuration.fields.POL.conversion]
name = "Polarity"
description = "Alert pin polarity"
active-low = 0
active-high = 1

[Configuration.fields.F]
description = "Fault queue"
base = "uint"
start = 3
end = 5

[Configuration.fields.F.conversion]
name = "fault-queue"
description = "Fault queue depth"
_1 = 0
_2 = 1
_4 = 2
_6 = 3

[Configuration.fields.R]
description = "Resolution"
access = "RO"
base = "uint"
start = 5
end = 7

[Configuration.fields.OS]
description = "One-shot"
base = "bool"
start = 7
end = 8

[Configuration.fields.EM]
description = "extended-mode"
base = "uint"
start = 12
end = 13

[Configuration.fields.EM.conversion]
name = "extended-mode"
description = "Extended mode"
disable = 0
enable = 1

[Configuration.fields.AL]
description = "Alert"
base = "bool"
start = 13
end = 14

[Configuration.fields.CR]
description = "Conversion rate"
base = "uint"
start = 14
end = 16

[Configuration.fields.CR.conversion]
name = "conversion-rate"
description = "Conversion rate"
_0_25Hz = 0
_1Hz = 1
_4Hz = 2
_8Hz = 3
```

A few nice things fall out of this immediately:

- `SD` stops being a magic bit and becomes a `ShutdownMode`
- `CR` stops being `0b10` and becomes `ConversionRate::_4Hz`
- read-only fields like `R` are encoded as such in the generated API

> [!TIP]
> Keep the TOML file focused on *register truth*, not ergonomic policy.
> The manifest should describe what the hardware is. The public driver
> API can then decide what feels pleasant and safe for humans.

## 4. Generate `src/inner.rs`

With the manifest in place, generate the low-level register interface:

```console
$ device-driver-cli -m tmp102.toml -d Inner -o src\inner.rs
```

The generated file is intentionally not the public API. We will treat it
as an implementation detail:

- `inner.rs` knows registers, fields, and access widths
- our hand-written wrapper will know addresses, conversions, and
  human-facing methods

That split is the sweet spot. Let the machine write the repetitive code;
keep the policy decisions for the part humans maintain.

Next we connect that generated layer to a real I<sup>2</sup>C bus.
