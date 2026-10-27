**Pagumie** is a high-performance, robust, and idiomatic Rust library and toolkit for the **Ethiopian calendar (Ge'ez calendar)**. It provides bidirectional date conversion (Ethiopian ⇄ Gregorian), date arithmetic, Ge'ez numeral formatting, and astronomical/computational calendar algorithms (Bahire Hasab / ባሕረ ሐሳብ).

The project is named after **Pagumē** (ጳጉሜ)—the unique 13th month of the Ethiopian calendar consisting of 5 epagomenal days in a standard year and 6 days in a leap year.

---

## Table of Contents

- [Features](#features)
- [Repository Structure](#repository-structure)
- [Installation](#installation)
- [Quick Start](#quick-start)
  - [1. Creating Dates](#1-creating-dates)
  - [2. Gregorian ⇄ Ethiopian Conversion](#2-gregorian--ethiopian-conversion)
  - [3. Ge'ez Numerals & Formatting](#3-geez-numerals--formatting)
  - [4. Leap Year & Pagumie Validation](#4-leap-year--pagumie-validation)
- [Repository Architecture](#repository-architecture)
  - [Workspace Crates](#workspace-crates)
  - [Fuzzing (`fuzz/`)](#fuzzing-fuzz)
  - [Reference Test Data (`reference/`)](#reference-test-data-reference)
- [Development & Testing](#development--testing)
- [Roadmap](#roadmap)
- [Contributing](#contributing)
- [License](#license)

---

## Features

- **Fast & Correct Conversion**: High-throughput Julian Day Number (JDN) algorithms for converting between Gregorian and Ethiopian calendars with zero allocations.
- **Full Calendar Validation**: Strict handling of month lengths (12 months of 30 days + Pagumē with 5 or 6 days).
- **Ge'ez Numeral Support**: Bi-directional conversion between Arabic integers (`1`, `24`, `1990`) and traditional Ge'ez script numerals (`፩`, `፳፬`, `፲፱፻፺`).
- **Bahire Hasab (ባሕረ ሐሳብ)**: Computations for moveable Christian feasts, fasts, and liturgical markers (e.g., *Nenewe*, *Fasika/Easter*, *Enkutatash*).
- **`no_std` Compatible**: Core algorithms are designed to operate without standard library allocations where needed.
- **Fuzz Tested**: Hardened against edge-case panics and integer overflows via continuous `cargo-fuzz` property testing.

---

## Repository Structure

```text
pagumie/
├── .github/
│   └── workflows/          # Continuous integration (fmt, clippy, test, fuzz)
├── crates/                 # Cargo workspace members
│   ├── pagumie-core/       # Core calendar math, conversion algorithms, primitives
│   └── pagumie/            # High-level ergonomics, formatting, and public facade
├── fuzz/                   # libFuzzer targets for stress testing converters
├── reference/              # Canonical test vectors, astronomical data, & reference scripts
├── Cargo.toml              # Workspace root manifest
├── rustfmt.toml            # Code style configuration
└── README.md
```

---

## Installation

Add `pagumie` to your `Cargo.toml`:

```toml
[dependencies]
pagumie = "0.1"
```

Or install using `cargo`:

```bash
cargo add pagumie
```

To point directly to the latest Git repository:

```toml
[dependencies]
pagumie = { git = "https://github.com/mitegab/pagumie.git", branch = "main" }
```

---

## Quick Start

### 1. Creating Dates

```rust
use pagumie::{EthiopianDate, Month};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create an Ethiopian date: Meskerem 1, 2017 (New Year / Enkutatash)
    let eth_date = EthiopianDate::from_ymd(2017, Month::Meskerem, 1)?;
    println!("Ethiopian Date: {}", eth_date);

    // Month lengths
    assert_eq!(Month::Meskerem.days(2017), 30);
    assert_eq!(Month::Pagumie.days(2017), 5); // 6 in leap year

    Ok(())
}
```

### 2. Gregorian ⇄ Ethiopian Conversion

```rust
use pagumie::{EthiopianDate, GregorianDate, Month};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // September 11, 2024 (Gregorian) -> Meskerem 1, 2017 (Ethiopian)
    let greg = GregorianDate::from_ymd(2024, 9, 11)?;
    let eth: EthiopianDate = greg.to_ethiopian();

    assert_eq!(eth.year(), 2017);
    assert_eq!(eth.month(), Month::Meskerem);
    assert_eq!(eth.day(), 1);

    // Convert back: Meskerem 1, 2017 -> September 11, 2024
    let back_to_greg: GregorianDate = eth.to_gregorian();
    assert_eq!(back_to_greg, greg);

    Ok(())
}
```

### 3. Ge'ez Numerals & Formatting

```rust
use pagumie::geez::{to_geez_numeral, from_geez_numeral};

fn main() {
    let year_num = 2017;
    let geez_str = to_geez_numeral(year_num);
    println!("Ge'ez Year: {}", geez_str); // ፳፻፲፯

    let parsed = from_geez_numeral(&geez_str).unwrap();
    assert_eq!(parsed, 2017);
}
```

### 4. Leap Year & Pagumie Validation

```rust
use pagumie::{is_ethiopian_leap_year, EthiopianDate, Month};

fn main() {
    // Ethiopian leap year occurs every 4 years without century exceptions
    assert!(is_ethiopian_leap_year(2015));
    assert!(!is_ethiopian_leap_year(2016));

    // Pagumie 6 is only valid during an Ethiopian leap year
    let valid_pagumie_6 = EthiopianDate::from_ymd(2015, Month::Pagumie, 6);
    assert!(valid_pagumie_6.is_ok());

    let invalid_pagumie_6 = EthiopianDate::from_ymd(2016, Month::Pagumie, 6);
    assert!(invalid_pagumie_6.is_err());
}
```

---

## Repository Architecture

### Workspace Crates

- **`crates/pagumie-core`**:
  - Implements the pure calendar conversion logic via Julian Day Number (JDN).
  - Minimal dependencies, `no_std` compliant.
- **`crates/pagumie`**:
  - End-user API with high-level features, custom formatters, localized month strings, and optional integrations (e.g., `serde`, `chrono`).

### Fuzzing (`fuzz/`)

Under `fuzz/`, cargo-fuzz targets verify invariant guarantees:
- **Round-trip fidelity**: `Gregorian -> Ethiopian -> Gregorian` must remain idempotent for all valid dates.
- **Overflow & boundary checking**: Arithmetic on leap years and epoch boundaries never panics.

Run the fuzz test suite:
```bash
cargo install cargo-fuzz
cargo fuzz run fuzz_roundtrip
```

### Reference Test Data (`reference/`)

The `reference/` directory contains ground-truth fixtures and benchmark data:
- Known Ethiopian Orthodox liturgical feast dates.
- Test vectors cross-referenced against historical calendars and astronomical almanacs.

---

## Development & Testing

Ensure you have a modern Rust toolchain installed:

```bash
# Clone the repository
git clone https://github.com/mitegab/pagumie.git
cd pagumie

# Build all workspace crates
cargo build --workspace

# Run all unit and integration tests
cargo test --workspace

# Check formatting and linting
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
```

---

## Roadmap

- [ ] `chrono` & `time` crate interoperability features
- [ ] Comprehensive Bahire Hasab (ባሕረ ሐሳብ) liturgical feasts calculator
- [ ] WebAssembly (`wasm32-unknown-unknown`) build targets and npm wrapper
- [ ] CLI utility for quick terminal date conversions

---

## Contributing

Contributions are welcome! Please follow these steps:

1. Fork the repository.
2. Create your feature branch (`git checkout -b feature/my-new-feature`).
3. Commit your changes (`git commit -m 'Add some feature'`).
4. Ensure tests and formatting pass (`cargo test && cargo fmt -- --check`).
5. Push to the branch (`git push origin feature/my-new-feature`).
6. Open a Pull Request.

---


```
