# mustermann

![CI](https://github.com/schultyy/mustermann/actions/workflows/ci.yml/badge.svg)
<br />
<img src="picture.jpeg" alt="Mustermann" width="300">

Mustermann is a CLI tool to generate random data to test OpenTelemetry pipelines .

## Installation

```bash
cargo install mustermann
```

Or install it from the [releases page](https://github.com/schultyy/mustermann/releases).

## Usage

### Logging

Logging to stdout:

```bash
cargo run -- --log stdout
```

Logging to an OTLP backend (default endpoint is `http://localhost:4317`):

```bash
cargo run -- --log otlp
```

Logging to a custom OTLP endpoint:

```bash
cargo run -- --log otlp --otlp-endpoint http://other-host:4317
```

Set a custom log level via `RUST_LOG` (works for stdout and OTLP):

```bash
RUST_LOG=DEBUG cargo run -- --log stdout
```

## License

MIT
