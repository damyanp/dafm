# DAFM

[![CI](https://github.com/damyanp/dafm/workflows/CI/badge.svg)](https://github.com/damyanp/dafm/actions)

A factory automation game built with the [Bevy](https://bevyengine.org/) game engine.

## Features

- Factory automation gameplay with conveyor belts, generators, and operators
- Interactive tile-based building system
- Real-time payload transport simulation
- Bridge systems for complex factory layouts

## Development

This project uses Rust nightly. Make sure you have the correct toolchain installed:

```bash
rustup toolchain install nightly
```

### Building

```bash
cargo build
```

### Running

```bash
cargo run
```

### Testing

```bash
cargo test
```

## System Requirements

- Linux: `libasound2-dev`, `libudev-dev`, and `pkg-config`
- macOS: No additional dependencies
- Windows: No additional dependencies

## License

Licensed under the MIT License. See [LICENSE](LICENSE) for details.