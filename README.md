

# SplitWrite – High-Concurrency Reader-Writer Pattern in Rust

SplitWrite is a Rust-based implementation of a **high-performance reader-writer synchronization primitive** tailored for **read-heavy workloads**. It maintains two versions of a data structure, isolating write operations and allowing multiple readers to access shared data in parallel **without locks**.

This approach significantly reduces contention and boosts scalability in multi-core environments.

---

## 📁 Folder Structure

```bash
c/
├── read/
│ ├── factory.rs # Factory pattern for creating read-access handles
│ ├── guard.rs # RAII guards for safe concurrent access
│ ├── aliasing.rs # Prevents aliasing violations
│ ├── lib.rs # Library entry point
│ ├── read.rs # Core read logic
│ ├── sync.rs # Synchronization primitives
│ ├── utilities.rs # Helper functions and common utilities
│ └── write.rs # Write path implementation
├── tests/
│ ├── deque.rs # Functional and integration tests
│ └── loom.rs # Deterministic testing with Loom for concurrency safety
Cargo.toml # Project metadata and dependencies

```
---

## 🚀 Features

- 🧵 Lock-free reads optimized for multi-threaded workloads
- 📝 Single-writer support for serialized mutations
- ⚙️ Zero runtime cost for readers during read-only phases
- 🧪 Loom integration for deterministic concurrency testing
- 📦 Modular, extensible codebase for embedding into larger systems

---

## 🛠️ Getting Started

### 📦 Prerequisites

- [Rust](https://www.rust-lang.org/tools/install) (1.65+ recommended)
- `cargo` for building and running the project

### 🔧 Build & Run

```bash
git clone https://github.com/rakeshrakhi9392/Splitwrite.git
cd SplitWrite
cargo build --release
To run tests:
cargo test

To run deterministic concurrency tests with Loom:
cargo test --test loom

```
## 💡 Design Philosophy

This project implements a concurrency pattern based on read/write separation. It uses:

Dual buffers or states: one for readers and one for writers

Swap-based synchronization: writes update the inactive state and trigger a swap

No reader locking: readers never block each other or the writer

Such separation enables near-linear read scalability on multi-core CPUs.

## 🔬 Testing & Verification

SplitWrite includes both standard and Loom-based concurrency tests to ensure safety under various interleavings:

tests/deque.rs: Functional and integration tests

tests/loom.rs: Deterministic model checking using Loom

##📚 Example Use Cases

In-memory cache layers in web services

Analytics dashboards where writes are infrequent

Real-time multiplayer game state replication

High-throughput event sourcing or pub/sub systems

## 📌 Future Work

 Generalize to support other collection types (e.g., Vec, HashMap)

 Add benchmarks comparing with RwLock, Mutex, etc.

 Optional async/await support for integration with async runtimes











