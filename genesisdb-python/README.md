# genesisblockdb-client

Python REST client for [GenesisBlockDB](https://github.com/Freshair129/GenesisBlock).

> This package is a **client for a running GenesisBlockDB server**. It does not embed the Rust database engine inside Python.

## Install from source

```bash
python -m pip install ./genesisdb-python
```

## Registry install

The intended PyPI distribution name is:

```bash
pip install genesisblockdb-client
```

Do not advertise that command as live until the first PyPI release has been published and verified from a clean environment.

The Python import namespace remains `genesisdb`:

```python
from genesisdb import GenesisClient

client = GenesisClient("http://localhost:3000")
node = client.add_node(
    labels=["Doc"],
    props={"title": "Hello from Python"},
)
print(node.id)
```

Start the standalone server from the repository with:

```bash
cargo run --release --no-default-features --features bins --bin genesis-db-server
```

The default server endpoint is `http://localhost:3000`.

## Development

Build wheel + source distribution:

```bash
cd genesisdb-python
python -m pip install --upgrade build
python -m build
```

Run unit tests:

```bash
python -m unittest discover -s tests -v
```

The repository CI also installs the built wheel into a clean Python environment and runs a live integration test against the Rust standalone server before the package is considered publishable.
