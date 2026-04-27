# pyco-de-gallo examples

Runnable Python scripts that demonstrate how to drive a Pico de Gallo
device from the `pyco_de_gallo` Python module.

## Setup

From the `crates/pyco-de-gallo` directory:

```sh
python -m venv .env
. .env/bin/activate          # Windows: .env\Scripts\Activate.ps1
pip install maturin
maturin develop --release
```

This builds the native extension and installs it into the active
virtual environment.

## Running

With the device plugged in:

```sh
python examples/tmp108_read.py
```

## Examples

| Script             | Peripheral | What it does                                                |
|--------------------|------------|-------------------------------------------------------------|
| `tmp108_read.py`   | I2C        | Reads ambient temperature from a TMP108 sensor at `0x48`.   |
