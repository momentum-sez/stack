# Contributing to Momentum SEZ Stack

Thank you for your interest in contributing to the Momentum SEZ Stack. This document provides guidelines for contributing to the project.

## Getting Started

### Prerequisites

Ensure you have Python 3.10 or higher installed, then set up your development environment:

```bash
git clone https://github.com/momentum-sez/stack.git
cd stack
pip install -r tools/requirements.txt
pytest -q
```

### Running Tests

Before submitting any changes, ensure all tests pass:

```bash
# Run all tests
pytest -q

# Run specific test file
pytest tests/test_mass_primitives.py -v

# Run with coverage
pytest --cov=tools tests/
```

## Contribution Guidelines

### Code Style

The project follows standard Python conventions with particular attention to documentation and type hints. All public functions should include docstrings referencing the relevant MASS Protocol specification section where applicable.

### Commit Messages

Write clear, descriptive commit messages that explain what changed and why. For version bumps, follow the format established in the repository (see the git history for examples).

### Pull Request Process

Before submitting a pull request, ensure your changes pass all existing tests and include new tests for any new functionality. Update relevant documentation including the README, CHANGELOG, and any affected specification documents. Reference any related issues in your pull request description.

### Specification Compliance

When implementing features from the MASS Protocol specification, include the relevant Definition, Protocol, Theorem, or Lemma reference in code comments and docstrings. This ensures traceability between the implementation and the formal specification.

### Schema Changes

When adding or modifying JSON schemas, ensure the schema validates correctly and update the schema count in documentation if adding new schemas. All schemas should be placed in the `schemas/` directory with appropriate naming conventions.

## Reporting Issues

When reporting bugs, include the version of the stack you're using, steps to reproduce the issue, expected behavior versus actual behavior, and any relevant error messages or logs.

## Questions

For questions about the project or specification, please open a GitHub issue with the "question" label.

## License

By contributing, you agree that your contributions will be licensed under the same terms as the project (see `LICENSES/` directory).
