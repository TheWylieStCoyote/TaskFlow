# Contributing to TaskFlow

Thank you for your interest in contributing to TaskFlow! This document provides guidelines and instructions for contributing.

## Getting Started

1. Fork the repository
2. Clone your fork: `git clone https://github.com/your-username/taskflow.git`
3. Create a new branch: `git checkout -b feature/your-feature-name`
4. Make your changes
5. Run tests: `cargo test`
6. Commit your changes with a descriptive message
7. Push to your fork: `git push origin feature/your-feature-name`
8. Open a Pull Request

## Development Setup

### Prerequisites

- Rust 1.87 or later (MSRV)
- Cargo

### Building

```bash
cargo build
```

### Running Tests

```bash
cargo test
```

### Running Lints

```bash
cargo clippy -- -D warnings
cargo fmt --check
```

### Running Benchmarks

```bash
cargo bench
```

Benchmark results are saved to `target/criterion/` with HTML reports.

## Code Style

- Follow Rust's standard formatting (`cargo fmt`)
- Address all Clippy warnings
- Write descriptive commit messages
- Add tests for new functionality
- Update documentation as needed

## Commit Messages

We follow conventional commits:

- `feat:` New features
- `fix:` Bug fixes
- `docs:` Documentation changes
- `test:` Adding or updating tests
- `refactor:` Code refactoring
- `ci:` CI/CD changes
- `chore:` Maintenance tasks

Example: `feat: add time tracking for tasks`

## Pull Request Process

1. Ensure all tests pass
2. Update documentation if needed
3. Add a clear description of changes
4. Link any related issues
5. Request review from maintainers

## Branching Strategy

- `master` / `main`: Stable release branch
- `dev`: Development branch
- `feature/*`: Feature branches

## Questions?

Feel free to open an issue for any questions or concerns.
