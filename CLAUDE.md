# Rust Project Conventions & Claude Directives

## 1. Core Philosophy

- Write idiomatic, memory-safe, and highly performant Rust code.
- Prefer compile-time safety and strict type system guarantees over runtime checks.
- Keep functions small, modular, and single-responsibility.
- **Declarative Style:** Always use declarative programming (iterators, functional combinators, pattern matching, expressions) over imperative loops and unnecessary mutable state.
- Declarative-First Approach: Default to declarative programming patterns (iterators, functional combinators, expressions, pattern matching) wherever possible. Avoid imperative control flow and mutable state, resorting to imperative code only when strictly unavoidable due to performance constraints or algorithmic limitations.

## 2. Code Style, Comments & Naming

- **Documentation Only:** Write documentation comments (`///`) for functions only. Do NOT add inline comments anywhere else in the code.
- **Self-Documenting Names:** Use expressive, precise, and well-thought-out names for functions, structs, enums, enum fields, and variables so code intent is clear without comments.
- **No Unwraps:** Never use `.unwrap()` or `.expect()` in production code. Explicitly handle all `Option` and `Result` types using `?`, `match`, or combinators (`map`, `and_then`).
- **Clippy Compliance:** Code MUST pass `cargo clippy --all-targets --all-features -- -D warnings` without any warnings.
- **Formatting:** Strictly adhere to `rustfmt` standards.
- **Traits vs Dyn:** Prefer generics/static dispatch (`impl Trait`) over dynamic dispatch (`dyn Trait`) unless runtime polymorphism is strictly required.

## 3. Error Handling

- **Option to Result Conversion:** Use `.ok_or()` for static/constant errors. Use `.ok_or_else()` when the error needs to be dynamically evaluated or constructed.
- Use `thiserror` for library, domain, or granular internal error definitions.
- Use `anyhow` only at application entry points (e.g., `main.rs`, CLI handlers, top-level API handlers).
- Always attach meaningful context to errors using `.context()` / `.with_context()`.

## 4. Async & Concurrency (Tokio)

- **Runtime:** Tokio is the standard async runtime for this project.
- **Mutex Selection:** Use `tokio::sync::Mutex` across async `.await` points. Use `std::sync::Mutex` ONLY for non-blocking, instantaneous standard thread locking.
- **Task Spawning:** Always handle handles returned by `tokio::spawn` and manage task cancellation/graceful shutdown explicitly.

## 5. Testing & Quality

- Include inline unit tests in a `#[cfg(test)] mod tests` block at the bottom of modified/created files.
- Ensure integration tests are placed under the `tests/` directory.
- Use `pretty_assertions` or standard `assert_eq!` for test assertions.

## 6. Execution Commands

When asked to verify or test code, run the following:

- Fast check: `cargo check`
- Lint: `cargo clippy -- -D warnings`
- Run tests: `cargo test`

## 7. Output Format Expectations

- Provide fully written, production-ready Rust code without skipping implementation details (avoid `// TODO: implement later`).
- Do not write verbose setup explanations before code blocks; lead directly with the solution or code changes.
