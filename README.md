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

## Config Syntax

```yaml
- task_name: App Logs
  frequency: Infinite
  template: "User %s logged in"
  vars:
    - Franz Josef
    - 34
    - Heinz
  severity: INFO
- task_name: App Login Errors
  frequency: Amount(45)
  template: "Failed to login: %s"
  vars:
    - Invalid username or password
    - Upstream connection refused
  severity: ERROR
```

## Services

A service is an evolution of the log file. While logs are very much standalone, services are interconnected.
I want to describe a service like this:

```yaml
services:
  - name: payments
    methods:
      - name: charge
        call: 
          name: checkout
          method: process
    interval_ms: 500
  - name: checkout
    methods:
      - name: process
        stdout: Processing Order
```

## License

MIT
