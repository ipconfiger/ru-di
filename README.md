# ru-di

A simple and lightweight dependency injection container for Rust.

[![Crates.io](https://img.shields.io/crates/v/ru-di.svg)](https://crates.io/crates/ru-di)
[![Documentation](https://docs.rs/ru-di/badge.svg)](https://docs.rs/ru-di)
[![License](https://img.shields.io/crates/l/ru-di.svg)](https://github.com/yourusername/ru-di/blob/main/LICENSE)

## Features

- Simple and lightweight
- Thread-safe
- Support for both transient and singleton services
- No runtime overhead
- Zero dependencies

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
ru-di = "0.1"
```

## Usage

### Basic Usage

```rust
use ru_di::Di;

// Define your services
struct Database {
    port: u16,
}

struct AppService {
    db: Database,
}

// Register services
Di::register::<Database, _>(|_| Database { port: 3306 });
Di::register::<AppService, _>(|di| {
    let db = di._get::<Database>().unwrap();
    AppService { db: db.clone() }
});

// Get service instance
let app = Di::get::<AppService>().unwrap();
assert_eq!(app.db.port, 3306);
```

### Singleton Services

```rust
use ru_di::Di;

#[derive(Debug, PartialEq)]
struct Configuration {
    port: u16,
}

// Register a singleton
Di::register_single(Configuration { port: 8080 });

// Get singleton instance
if let Some(mut config) = Di::get_single::<Configuration>() {
    let config = config.get_mut();
    assert_eq!(config.port, 8080);
    config.port = 8081;
}

// The change persists
if let Some(mut config) = Di::get_single::<Configuration>() {
    let config = config.get_mut();
    assert_eq!(config.port, 8081);
}
```

## API Documentation

### Registering Services

- `Di::register<T, F>(factory: F)` - Register a transient service
- `Di::register_single<T>(instance: T)` - Register a singleton service

### Getting Services

- `Di::get<T>() -> Result<T, Box<dyn Error>>` - Get a transient service instance
- `Di::get_single<T>() -> Option<SingleRef<T>>` - Get a singleton service instance

## Thread Safety

All operations are thread-safe. The container uses `Arc`, `Mutex`, and `RwLock` internally to ensure thread safety.

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details. 