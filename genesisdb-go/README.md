# GenesisBlockDB Go SDK

Go REST client for the GenesisBlockDB standalone server.

## Module path

```text
github.com/Freshair129/GenesisBlock/genesisdb-go
```

This SDK is a Go **submodule inside the GenesisBlock monorepo**. The module path therefore includes the `genesisdb-go` directory.

## Development from source

```bash
git clone https://github.com/Freshair129/GenesisBlock.git
cd GenesisBlock/genesisdb-go
go test ./...
```

## Public module install

Once a Go SDK release tag has been cut, consumers install with:

```bash
go get github.com/Freshair129/GenesisBlock/genesisdb-go@v0.1.0
```

Submodule release tags use the Go-compatible repository tag form:

```text
genesisdb-go/v0.1.0
```

Do not advertise a version as released until that tag exists and a clean external consumer resolves it successfully.

## Usage

```go
package main

import (
    "context"

    genesisdb "github.com/Freshair129/GenesisBlock/genesisdb-go"
)

func main() {
    client := genesisdb.NewClient("http://localhost:3000")
    node, err := client.AddNode(context.Background(), genesisdb.NodeInput{
        Labels: []string{"Doc"},
        Props: map[string]interface{}{"title": "Hello from Go"},
    })
    if err != nil {
        panic(err)
    }
    println(node.ID)
}
```

The GenesisBlockDB server must be running separately. From the repository root:

```bash
cargo run --release --no-default-features --features bins --bin genesis-db-server
```
