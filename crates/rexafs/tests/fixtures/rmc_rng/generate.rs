//! Historical fixture generator. Compile in a separate Cargo project with
//! rand = "=0.9.5", rand_chacha = { version = "=0.9.0", features = ["serde"] },
//! and serde_json = "=1.0.145". Write stdout to a new file for comparison;
//! do not overwrite the retained fixture when changing dependencies.
use rand::{seq::SliceRandom, Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;
use serde_json::json;

fn main() {
    let mut cases = Vec::new();
    for (seed, stream, word_pos) in [
        (0_u64, 0_u64, 0_u128),
        (17, 5, 1),
        (20260918, 123, 63),
        (u64::MAX, u64::MAX, 127),
    ] {
        let mut rng = ChaCha8Rng::seed_from_u64(seed);
        let seeded = serde_json::to_value(&rng).unwrap();
        rng.set_stream(stream);
        rng.set_word_pos(word_pos);
        let before = serde_json::to_value(&rng).unwrap();
        let samples: Vec<_> = (0..64)
            .map(|i| {
                let upper: usize = [1, 2, 3, 7, 32, 108, 256, 1009][i % 8];
                let atom = rng.random_range(0..upper);
                let coordinate_bits = rng.random::<f64>().to_bits();
                let crossover = rng.random::<bool>();
                json!([atom, coordinate_bits, crossover])
            })
            .collect();
        let mut shuffled: Vec<usize> = (0..24).collect();
        shuffled.shuffle(&mut rng);
        cases.push(json!({
            "seed": seed, "seeded": seeded, "before": before,
            "samples": samples, "shuffled": shuffled, "after": rng
        }));
    }
    println!("{}", json!({
        "rand": "0.9.5", "rand_chacha": "0.9.0", "cases": cases
    }));
}
