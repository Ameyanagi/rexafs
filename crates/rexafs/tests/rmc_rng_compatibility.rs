//! Protect the persisted ChaCha8 state and sampled values used by RMC and EA.
//! The retained fixture was generated with rand 0.9.5 / rand_chacha 0.9.0;
//! it must not be regenerated from the dependencies under test.
use rand::{seq::SliceRandom, RngExt, SeedableRng};
use rand_chacha::ChaCha8Rng;
use serde_json::{json, Value};

#[test]
fn historical_rmc_random_streams_continue_with_identical_values() {
    let fixture: Value =
        serde_json::from_str(include_str!("fixtures/rmc_rng/rand-0.9.json")).unwrap();
    for case in fixture["cases"].as_array().unwrap() {
        let seed = case["seed"].as_u64().unwrap();
        assert_eq!(
            serde_json::to_value(ChaCha8Rng::seed_from_u64(seed)).unwrap(),
            case["seeded"],
            "seed expansion changed for {seed}"
        );
        let mut rng: ChaCha8Rng = serde_json::from_value(case["before"].clone()).unwrap();
        let mut cloned = rng.clone();
        assert_eq!(serde_json::to_value(&rng).unwrap(), case["before"]);
        for (i, expected) in case["samples"].as_array().unwrap().iter().enumerate() {
            let upper: usize = [1, 2, 3, 7, 32, 108, 256, 1009][i % 8];
            for source in [&mut rng, &mut cloned] {
                let atom = source.random_range(0..upper);
                let coordinate_bits = source.random::<f64>().to_bits();
                let crossover = source.random::<bool>();
                assert_eq!(
                    json!([atom, coordinate_bits, crossover]),
                    *expected,
                    "RMC/EA draw changed for seed {seed}, sample {i}"
                );
            }
        }
        let mut shuffled: Vec<usize> = (0..24).collect();
        shuffled.shuffle(&mut rng);
        assert_eq!(serde_json::to_value(shuffled).unwrap(), case["shuffled"]);
        assert_eq!(serde_json::to_value(rng).unwrap(), case["after"]);
    }
}
