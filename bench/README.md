# Benchmarks

## Setting up

In this directory run:

```
bash create_fixtures.sh
```

Run a TFTP server with file directory set as `fixtures` directory or copy the files
to the correct location.

## Running

Build the library first:

```
cargo build
```

Then run the benchmark (this will benchmark the client using an `octet` transfer mode
and `get` request type):

```
bash bench_client.sh octet get
```
