# Contributing to PyReverseETL

Thanks for your interest in contributing! This guide will help you get started.

## Getting Started

1. **Fork the repository** on GitHub
2. **Clone your fork** locally
   ```bash
   git clone https://github.com/YOUR_USERNAME/PyReverseETL.git
   cd PyReverseETL
   ```
3. **Create a virtual environment**
   ```bash
   python -m venv venv
   source venv/bin/activate  # On Windows: venv\Scripts\activate
   ```
4. **Install in development mode**
   ```bash
   pip install -e ".[dev]"
   ```
5. **Run tests to verify setup**
   ```bash
   pytest tests/
   ```

## Making Changes

- **Create a feature branch**: `git checkout -b feature/your-feature-name`
- **Keep commits atomic**: One logical change per commit
- **Write tests** for new functionality
- **Run tests locally** before pushing
- **Follow PEP 8** style guidelines

## Submitting a Pull Request

1. Push to your fork
2. Create a Pull Request with a clear description
3. Reference any related issues (e.g., "Fixes #123")
4. Ensure tests pass

## Reporting Issues

- Use [GitHub Issues](https://github.com/Mullassery/PyReverseETL/issues)
- Include Python version, OS, and error messages
- Provide minimal reproducible example if possible

## Questions?

Open a [GitHub Discussion](https://github.com/Mullassery/PyReverseETL/discussions) for questions and ideas.

---

**License:** MIT — By contributing, you agree your work is licensed under the same terms.
