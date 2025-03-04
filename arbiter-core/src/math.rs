//! `math` module provides utility functions and structures for deterministic
//! mathematical operations and conversions commonly required for smart contract
//! and blockchain operations. This includes fixed-point conversions (WAD) and
//! seeded random number generation with a Poisson distribution.
//!
//! The main feature is the [`SeededPoisson`] struct which provides seeded
//! randomness for determining block sizes in a simulation. We also re-export
//! the [`RustQuant::stochastics`] module so that the end user may retrieve
//! stochastic processes of their choosing in a simulation they build.

#![warn(missing_docs, unsafe_code)]
use rand::{distributions::Distribution, rngs::StdRng, SeedableRng};
use statrs::distribution::Poisson;

/// Represents a Poisson distribution with a seeded random number generator.
///
/// This is useful for generating deterministic random values from a Poisson
/// distribution, given the same `rate_parameter` and `seed`.
/// The Poisson distribution is used in modeling the number of events that occur
/// over a fixed amount of time. It can also be used to model queue times as
/// well. For more detail, see the
/// [Wikipedia page](https://en.wikipedia.org/wiki/Poisson_distribution).
/// You may find there that the `rate_parameter` is denoted by the Greek letter
/// lambda.
///
/// The way we use it in `arbiter-core` is to give a random model for
/// the amount of transactions that go through a block. For instance, the larger
/// the `rate_paramater`, the more transactions we expect (on average) to fit
/// into a block. A large `rate_parameter` would represent a high-volume network
/// where lots of transactions are occurring. This could be during periods of
/// times of high market (DEX) volatility or during new NFT launches.
#[derive(Debug, Clone)]
pub struct SeededPoisson {
    /// Poisson distribution.
    pub distribution: Poisson,

    /// Time step for the Poisson distribution.
    pub time_step: u32,

    /// Random number generator.
    rng: StdRng,
}

impl SeededPoisson {
    /// Constructs a new [`SeededPoisson`] with the given `rate_parameter`
    /// (average rate of events) and a seed for the random number generator.
    ///
    /// # Arguments
    ///
    /// * `rate_parameter` - The average rate of events for the Poisson
    ///   distribution.
    /// * `seed` - The seed value for the random number generator.
    ///
    /// # Returns
    ///
    /// A new [`SeededPoisson`] instance.
    ///
    /// # Examples
    ///
    /// ```
    /// # use arbiter_core::math::SeededPoisson;
    /// let poisson = SeededPoisson::new(10.0, 12, 12345);
    /// ```
    pub fn new(rate_parameter: f64, time_step: u32, seed: u64) -> Self {
        let distribution = Poisson::new(rate_parameter).unwrap();
        let rng = StdRng::seed_from_u64(seed);
        Self {
            distribution,
            time_step,
            rng,
        }
    }

    /// Samples a single value from the Poisson distribution using the seeded
    /// random number generator.
    ///
    /// # Returns
    ///
    /// A random value sampled from the Poisson distribution.
    ///
    /// # Examples
    ///
    /// ```
    /// # use arbiter_core::math::SeededPoisson;
    /// let mut poisson = SeededPoisson::new(10.0, 12, 12345);
    /// let random_value = poisson.sample();
    /// ```
    pub fn sample(&mut self) -> usize {
        self.distribution.sample(&mut self.rng) as usize
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn seeded_poisson() {
        let mut test_dist_1 = SeededPoisson::new(10.0, 10, 321);
        let mut test_dist_2 = SeededPoisson::new(10000.0, 11, 123);
        let mut test_dist_3 = SeededPoisson::new(10000.0, 12, 123);

        let result_1 = test_dist_1.sample();
        let result_2 = test_dist_1.sample();
        let result_3 = test_dist_2.sample();
        let result_4 = test_dist_2.sample();
        let result_5 = test_dist_3.sample();
        let result_6 = test_dist_3.sample();

        assert_eq!(result_1, 15);
        assert_eq!(result_2, 12);
        assert_eq!(result_3, 9914);
        assert_eq!(result_4, 10143);
        assert_eq!(result_5, result_3);
        assert_eq!(result_6, result_4);
    }
}
