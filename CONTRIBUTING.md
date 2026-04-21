# Contributing to Smart Clipboard

Thanks for contributing to Smart Clipboard.

## Development setup

```bash
pnpm install
pnpm tauri dev
```

## Quality workflow

This project uses local Git hooks and CI to keep code quality consistent.

### Local hooks

- `pre-commit`: formats staged files with Prettier and rustfmt
- `commit-msg`: validates commit messages with commitlint
- `pre-push`: runs the main quality gate before pushing

### Main commands

```bash
pnpm run format
pnpm run format:check
pnpm run lint
pnpm run typecheck
pnpm run test:web
pnpm run test:web:coverage
pnpm run test:rust
pnpm run check
```

## Commit message convention

Use Conventional Commits when possible:

- `feat: add xxx`
- `fix: resolve xxx`
- `test: add xxx`
- `ci: improve xxx`
- `chore: update xxx`

## Pull requests

Before opening a PR, make sure:

1. `pnpm run check` passes locally
2. changes include tests when practical
3. docs are updated if behavior changes
4. commit messages follow the configured convention

## Testing guidance

- Prefer unit tests for stores, composables, and utility modules
- Keep UI tests focused and stable
- For Rust changes, run `pnpm run test:rust`
- For frontend changes, run `pnpm run test:web`
