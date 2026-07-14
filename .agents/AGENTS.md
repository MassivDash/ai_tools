# Workspace Rules & Verification Pipeline

## Test-Driven Development (TDD)
You MUST follow a Test-Driven Development (TDD) approach when developing features or fixing bugs:
1. **Write a failing test** first that defines the expected behavior of the new feature or describes the bug.
2. **Implement the minimum code** required to make the test pass.
3. **Refactor** the code while ensuring all tests continue to pass.

## Core Principles (YAGNI & KISS)
- **KISS (Keep It Simple, Stupid)**: Avoid over-engineering. Favor readable, direct, and simple code over complex or overly clever abstractions.
- **YAGNI ("We will not need that in the future")**: Do not write code or add hooks under the assumption that they might be useful later. Only implement what is strictly necessary to satisfy the current requirements.

## Verification Pipeline
After completing any feature, bug fix, or codebase modification, you MUST run the verification pipeline to ensure everything is building, formatted, linted, and tested. Set `set -e` to exit immediately if any command fails.

Run the following commands in the workspace root:

```bash
# 1. Verify CLI Runner
echo '🧪 running cli cargo test...'
cargo test

echo '🧹 running cli runner cargo fmt...'
cargo fmt 

echo '🔍 running cli cargo clippy...'
cargo clippy -- -D warnings

# 2. Verify Backend (Actix Web)
echo '🧪 running backend cargo test...'
cargo test --manifest-path src/backend/Cargo.toml

echo '🧹 running backend cargo fmt...'
cargo fmt --manifest-path src/backend/Cargo.toml

echo '🔍 running backend cargo clippy...'
cargo clippy --manifest-path src/backend/Cargo.toml -- -D warnings

# 3. Verify Frontend (Astro/Svelte)
echo '🧹 running frontend project lint...'
npm run lint:all --prefix src/frontend/

echo '🧪 running frontend vitest...'
npm run test --prefix src/frontend/

echo '🧪 running astro check...'
npm run build --prefix src/frontend/

echo '------ Done ------'
```

Do not mark the task as complete or present the changes for final review until all verification steps execute successfully with zero warnings/errors.
