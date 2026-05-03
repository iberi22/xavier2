# Contributing to Xavier2

Thank you for your interest in contributing to Xavier2!

## Quick Links

- [Code of Conduct](./docs/CODE_OF_CONDUCT.md)
- [Security Policy](./docs/SECURITY.md)
- [Architecture Overview](./docs/ARCHITECTURE/README.md)

## How Can I Contribute?

### Reporting Bugs

Before creating a bug report:
1. Check the [existing issues](https://github.com/iberi22/xavier2/issues)
2. Use the bug report template in `.github/ISSUE_TEMPLATE/bug.md`
3. Include:
   - Your operating system and version
   - Xavier2 version (`xavier2 --version` or `xavier2 stats`)
   - Clear steps to reproduce
   - Actual vs expected behavior

### Suggesting Features

1. Search [existing feature requests](https://github.com/iberi22/xavier2/labels/enhancement)
2. Use the feature request template in `.github/ISSUE_TEMPLATE/feature.md`
3. Explain:
   - The problem you're solving
   - How your proposed solution addresses it
   - Any alternatives you've considered

### Pull Requests

1. **Fork** the repository
2. **Sync from `main`** and create a short-lived branch for your feature or fix:
   ```bash
   git fetch origin main
   git checkout -b feature/my-new-feature origin/main
   # or
   git checkout -b fix/issue-number origin/main
   ```
3. **Make your changes** following the coding standards below
4. **Add tests** for any new functionality
5. **Ensure tests pass**:
   ```bash
   cargo test --all
   cargo clippy --all-targets --all-features
   ```
6. **Commit** using conventional commits format (see below)
7. **Push** and create a Pull Request

### Branch Policy

- `main` is the default branch and should stay releasable.
- Use short-lived topic branches for every change.
- Prefer small pull requests with a clear scope.
- Delete merged branches after the pull request is closed.

## Coding Standards

### Rust Style
- Run `cargo fmt` before committing
- Run `cargo clippy --all-targets --all-features` and address warnings
- Follow [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)

### Commit Message Format
We follow [Conventional Commits](https://www.conventionalcommits.org/):

```
<type>(<scope>): <description>

[optional body]

[optional footer]
```

**Types:**
- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation changes
- `refactor`: Code refactoring
- `test`: Adding or updating tests
- `chore`: Maintenance tasks

**Examples:**
```
feat(memory): add memory consolidation API
fix(security): prevent path traversal in code/scan
docs(readme): update installation instructions
```

### Testing Requirements
- All new code must have tests
- Unit tests in `src/**/*tests*.rs` or `mod tests {}` blocks
- Integration tests in `tests/` directory
- Run `cargo test` before submitting PR

## Project Structure

```
xavier2/
├── src/              # Source code
├── tests/            # Integration tests
├── docs/              # Documentation
├── scripts/           # Utility scripts
└── .github/           # GitHub configuration
```

## Getting Help

- Open an issue for bugs or feature requests
- Check [docs/](docs/) for documentation
- Review [ROADMAP.md](docs/ROADMAP.md) for development plans

## License

By contributing, you agree that your contributions will be licensed under the MIT License.
