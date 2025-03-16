# mustermann

![CI](https://github.com/schultyy/mustermann/actions/workflows/ci.yml/badge.svg)
<br />
<img src="picture.jpeg" alt="Mustermann" width="300">

Mustermann is a CLI tool to generate random data to test OpenTelemetry pipelines.

## Installation

```bash
cargo install mustermann
```

Or install it from the [releases page](https://github.com/schultyy/mustermann/releases).

## Usage

Mustermann uses a simple domain-specific language (DSL) to define log generation patterns. A file with this language is called a "muster".

### Basic Structure

A basic muster file looks like this:

```
logs {
  application_name: "checkout-service"
  duration: indefinite         // How long to run: indefinite, 10m, 1h, etc.
  frequency: 100               // How often to generate logs, in ms

  templates: (                  // Log message templates with severity levels
    { severity: "INFO", message: "Performed checkout for $user.id" },
    { severity: "ERROR", message: "Checkout failed. Reason: $reason" }
  )

  data: (                       // Data to populate the templates
    user: {
      name: fake::name::Name(),
      id: fake::uuid::UUIDv4()
    },
    reason: fake::lorem::Sentence(1..2)
  )
}
```

### Severity Levels

Each log template requires a severity level. Supported levels are:

- TRACE
- DEBUG
- INFO
- WARN
- ERROR
- FATAL

### Templates

Templates are strings with placeholders that will be filled with generated data. Placeholders start with `$` and can reference any field defined in the `data` section:

```
templates: (
  { severity: "INFO", message: "User $user.name logged in from $location.city" },
  { severity: "WARN", message: "High latency detected: $metrics.latency ms" }
)
```

### Data Generation

#### Static Data

You can define static data directly:

```
data: (
  service: "payment-processor",
  version: "1.2.3"
)
```

#### Using fake-rs

Mustermann integrates with [fake-rs](https://github.com/cksac/fake-rs) for rich data generation:

```
data: (
  customer: {
    first_name: fake::name::FirstName(),
    last_name: fake::name::LastName(),
    email: fake::internet::SafeEmail()
  },
  transaction: {
    id: fake::uuid::UUIDv4(),
    amount: fake::number::NumberWithFormat("###.##"),
    currency: fake::currency::CurrencyCode()
  }
)
```

#### Localized Data

Specify locales for internationalized data:

```
data: (
  german_customer: {
    name: fake::name::Name(locale: "de_DE"),
    address: fake::address::StreetName(locale: "de_DE")
  }
)
```

### Template Weighting

Control the relative frequency of different log messages:

```
templates: (
  { severity: "INFO", message: "Normal operation", weight: 10 },
  { severity: "WARN", message: "Slow database query", weight: 3 },
  { severity: "ERROR", message: "Connection failed", weight: 1 }
)
```

With this configuration, "Normal operation" appears most frequently, followed by "Slow database query", with "Connection failed" being the least common.

### Output Configuration

Specify how and where logs are written:

```
logs {
  // ... other configuration ...

  output: {
    format: "json",           // json, plaintext, or custom
    destination: "stdout"     // stdout, file, or http
    file_path: "logs.txt"     // Only required if destination is "file"
    endpoint: "http://localhost:4318/v1/logs"  // Only if destination is "http"
  }
}
```

### Advanced Features

#### Conditional Templates

Generate logs only when specific conditions are met:

```
templates: (
  {
    severity: "ERROR",
    message: "Payment declined: $transaction.decline_reason",
    when: "$transaction.status == 'declined'"
  }
)
```

#### Sequences and Flows

Define sequences of logs that simulate a process flow:

```
flows: (
  checkout: [
    { severity: "INFO", message: "User $user.id started checkout" },
    { severity: "INFO", message: "Processing payment for $user.id", delay: "500ms" },
    {
      severity: "ERROR",
      message: "Payment failed for $user.id: $error.message",
      probability: 0.1,
      data: { error: { message: fake::payment::ErrorMessage() } }
    },
    { severity: "INFO", message: "Checkout completed for $user.id", when: "$error == null" }
  ]
)
```

#### Time-based Patterns

Simulate realistic traffic patterns:

```
patterns: {
  business_hours: {
    schedule: "Mon-Fri 09:00-17:00",
    multiplier: 5.0  // 5x normal frequency during business hours
  },
  maintenance_window: {
    schedule: "Sat 02:00-04:00",
    multiplier: 0.2  // Reduced traffic during maintenance
  }
}
```

#### Correlation Context

Include correlation IDs to link related logs:

```
context: {
  trace_id: fake::uuid::UUIDv4(),    // Generated once per session
  span_id: fake::uuid::UUIDv4(),     // Generated for each log or flow
  user_session: "$user.id",
  persistent: true                   // Keep the same context across logs
}
```

### Multiple Log Sources

Define multiple log sources in a single muster file:

```
logs {
  application_name: "api-gateway"
  // Configuration for API gateway logs
}

logs {
  application_name: "payment-service"
  // Configuration for payment service logs
}
```

### Complete Example

Here's a comprehensive example combining various features:

```
logs {
  application_name: "e-commerce-service"
  duration: 1h
  frequency: 50

  templates: (
    { severity: "INFO", message: "User $user.name ($user.id) logged in", weight: 10 },
    { severity: "INFO", message: "Added item $item.name to cart for user $user.id", weight: 8 },
    { severity: "INFO", message: "Processed payment of $payment.amount $payment.currency for order $order.id", weight: 5 },
    { severity: "WARN", message: "Unusual activity detected for user $user.id from IP $user.ip", weight: 2 },
    {
      severity: "ERROR",
      message: "Payment declined: $payment.decline_reason",
      weight: 1,
      when: "$payment.status == 'declined'"
    }
  )

  flows: (
    checkout: [
      { severity: "INFO", message: "Checkout initiated by user $user.id" },
      { severity: "INFO", message: "Validating shipping information" },
      { severity: "INFO", message: "Processing payment of $payment.amount $payment.currency" },
      {
        severity: "ERROR",
        message: "Payment processing failed: $payment.error",
        probability: 0.15,
        data: { payment: { error: fake::lorem::Sentence(1..2), status: "declined" } }
      },
      {
        severity: "INFO",
        message: "Order $order.id confirmed and placed",
        when: "$payment.status != 'declined'"
      }
    ]
  )

  data: (
    user: {
      id: fake::uuid::UUIDv4(),
      name: fake::name::Name(),
      email: fake::internet::SafeEmail(),
      ip: fake::internet::IPv4()
    },
    item: {
      id: fake::uuid::UUIDv4(),
      name: fake::commerce::ProductName(),
      price: fake::number::NumberWithFormat("##.##")
    },
    order: {
      id: fake::number::NumberWithFormat("ORD-#####"),
      items: fake::number::Digit(),
      total: fake::number::NumberWithFormat("###.##")
    },
    payment: {
      amount: "$order.total",
      currency: fake::currency::CurrencyCode(),
      status: fake::utils::either("approved", "declined", "processing"),
      decline_reason: fake::payment::DeclineReason()
    }
  )

  context: {
    trace_id: fake::uuid::UUIDv4(),
    span_id: fake::uuid::UUIDv4()
  }

  patterns: {
    peak_hours: {
      schedule: "* 10:00-14:00,18:00-22:00",
      multiplier: 2.5
    }
  }

  output: {
    format: "json",
    destination: "file",
    file_path: "e-commerce-logs.json"
  }
}
```

## License

MIT
