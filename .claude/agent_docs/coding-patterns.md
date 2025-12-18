# Coding Patterns

## Comment Style (Suckless-Lite)

We follow a minimalist approach to comments:

### When NOT to Comment
- Code that is self-explanatory (the default case)
- Comments that only make sense in current context or relative to past history
- Obvious operations (e.g., "increment counter", "loop through items")

### When to Comment
- Non-obvious algorithms or workarounds
- External constraints or requirements that aren't apparent from code
- Hardware-specific behavior or magic numbers from specs

### Comment Capitalization
- Always start comments with lowercase
- Exception: abbreviations (e.g., "PCI", "EFI", "DMA")

```rust
// bad
// This function handles input
// Increment the counter

// good
// work around ACPI bug in older BIOSes
// PCI BAR0 offset per spec v2.1
```

## Code Style

### Prefer Self-Explanatory Code
Make the code readable without comments:

```rust
// bad
let x = 42; // max retries

// good
let max_retries = 42;
```

### Early Returns Over Nesting

```rust
// bad
fn process(input: Option<Data>) -> Result<()> {
    if let Some(data) = input {
        if data.is_valid() {
            // ... deep nesting
        }
    }
}

// good
fn process(input: Option<Data>) -> Result<()> {
    let data = input.ok_or(Error::NoInput)?;
    if !data.is_valid() {
        return Err(Error::Invalid);
    }
    // ... flat code
}
```

### Keep Functions Small
- Under 20 lines when possible
- Single responsibility per function
- If a function needs comments to explain sections, split it

## Error Handling

### Be Direct, Not Defensive
Trust internal code and framework guarantees. Only validate at boundaries:

```rust
// bad - unnecessary validation
fn internal_process(data: &ValidatedData) -> Result<()> {
    if data.items.is_empty() {  // already validated at input
        return Err(Error::Empty);
    }
    // ...
}

// good - trust the type system
fn internal_process(data: &ValidatedData) -> Result<()> {
    for item in &data.items {
        // ...
    }
}
```

## Simplicity

### No Premature Abstraction
Three similar lines of code is better than a premature abstraction:

```rust
// bad - abstraction for one use case
fn create_path(base: &str, component: &str) -> PathBuf {
    PathBuf::from(base).join(component)
}

// good - just do it inline
let path = PathBuf::from("/usr/lib").join("foo.so");
```

### No Hypothetical Future Requirements
Design for current needs, not imagined futures:

```rust
// bad - feature flags for one implementation
struct Config {
    use_new_parser: bool,  // "might need old parser later"
}

// good - just use the implementation you need
struct Config {
    // actual config fields
}
```

## Build Scripts

### Keep Build Scripts Minimal
- Standard configure/make/install pattern
- Only add workarounds when needed (document why)
- Always end with the reproducibility epilogue

### Document Workarounds

```bash
# bad
CFLAGS=${CFLAGS/-flto=auto/}

# good
# disable LTO - causes linker issues with this package
CFLAGS=${CFLAGS/-flto=auto/}
```
