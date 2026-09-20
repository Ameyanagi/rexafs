# Historical RMC random-stream reference

`rand-0.9.json` was generated on 21 September 2026 (Asia/Tokyo), using Rust
1.98.1 on Apple Silicon, `rand 0.9.5`, `rand_chacha 0.9.0` with its `serde`
feature, and `serde_json 1.0.145`. These are the random-number dependencies of
released rexafs 0.2.12. This synthetic reference contains no measurement data.

The retained `generate.rs` records four seeds, initial serialized ChaCha8 states,
and states at different stream/word positions. Each case draws 64 sequences of
an atom index, a uniform float and a Boolean, followed by a 24-index shuffle.
Floats are stored as their 64-bit representations so comparisons do not depend
on decimal formatting. These operations cover the draws used by RMC proposals,
acceptance, evolutionary crossover and structure sampling. They are numerical
compatibility checks, not statistical tests of randomness or fit quality.

The integration test `rmc_rng_compatibility.rs` checks seed expansion, historical
state deserialization, clone behavior, every sampled value, shuffle order and
the final serialized state. It runs against the current dependencies; the
fixture must remain from the recorded older versions. Existing RMC session tests
separately cover full calculation and checkpoint continuation.

To reproduce the reference, copy `generate.rs` to `src/main.rs` in a new, isolated
Cargo project with this manifest, then run `cargo run --quiet` and compare stdout
with the retained JSON. Do not regenerate the fixture during an upgrade.

```toml
[package]
name = "rexafs-rng-fixture-generator"
version = "0.0.0"
edition = "2021"

[dependencies]
rand = { version = "=0.9.5", default-features = false, features = ["std"] }
rand_chacha = { version = "=0.9.0", features = ["serde"] }
serde_json = "=1.0.145"
```

The [Rand 0.10 migration guide](https://rust-random.github.io/book/update-0.10.html)
describes the trait renames. rexafs retains the explicit `rand_chacha::ChaCha8Rng`
type and its serialization support rather than switching to `StdRng`.
