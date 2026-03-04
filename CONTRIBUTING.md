# Contributing to Atropos 🦀

First off, thank you for considering contributing to Atropos! It's people like you that make this tool great.

## 📜 Code of Conduct
This project adheres to the [Rust Code of Conduct](https://www.rust-lang.org/policies/code-of-conduct).

## 🛠 Development Workflow

### Coding Standards
1.  **SOLID Principles:** We prioritize maintainability. Ensure your changes follow Single Responsibility and Dependency Inversion.
2.  **Strong Typing:** Avoid "Primitive Obsession." Use Newtypes for IDs and Enums for states.
3.  **Documentation:** All public functions should have Doc Comments (`///`).
4.  **Tests:** New features must include unit tests in the same file or integration tests in `/tests`.

### Pre-PR Checklist
Before submitting a Pull Request, ensure the following commands pass:

```bash
# Format code
cargo fmt --all

# Run linter
cargo clippy -- -D warnings

# Run tests
cargo test
```

### Pull Request Process
1.  Fork the repository and create your branch from `main`.
2.  Ensure your commit messages follow [Conventional Commits](https://www.conventionalcommits.org/).
3.  Update the `README.md` if you are introducing a new feature or changing an API.
4.  Once the CI passes, a maintainer will review your code.

## 🏗 Architectural Decisions
If you are proposing a significant architectural change, please open an **RFC Issue** first to discuss the design with the core maintainers. We value the **"Research -> Strategy -> Execution"** lifecycle.

---

*Happy Coding!*
