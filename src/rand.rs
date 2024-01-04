//! A simple and fast random number generator.
//!
//! The implementation uses [Wyrand](https://github.com/wangyi-fudan/wyhash), a simple and fast
//! generator but **not** cryptographically secure.
//!
//! Vendored from <https://github.com/smol-rs/fastrand> (MIT/APACHE-2.0).
use std::{
    cell::Cell,
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
    ops::{Bound, RangeBounds},
    thread,
    time::Instant,
};

/// A random number generator.
#[derive(PartialEq, Eq)]
pub struct Rng(Cell<u64>);

impl Default for Rng {
    #[inline]
    fn default() -> Rng {
        Rng::new()
    }
}

impl Clone for Rng {
    /// Clones the generator by deterministically deriving a new generator based on the initial seed.
    fn clone(&self) -> Rng {
        Rng::with_seed(self.gen_u64())
    }
}

impl Rng {
    /// Generates a random `u32`.
    #[inline]
    fn gen_u32(&self) -> u32 {
        self.gen_u64() as u32
    }

    /// Generates a random `u64`.
    #[inline]
    fn gen_u64(&self) -> u64 {
        let s = self.0.get().wrapping_add(0xA0761D6478BD642F);
        self.0.set(s);
        let t = u128::from(s) * u128::from(s ^ 0xE7037ED1A0B428DB);
        (t as u64) ^ (t >> 64) as u64
    }

    /// Generates a random `u128`.
    #[inline]
    fn gen_u128(&self) -> u128 {
        (u128::from(self.gen_u64()) << 64) | u128::from(self.gen_u64())
    }

    /// Generates a random `u32` in `0..n`.
    #[inline]
    fn gen_mod_u32(&self, n: u32) -> u32 {
        // Adapted from: https://lemire.me/blog/2016/06/30/fast-random-shuffling/
        let mut r = self.gen_u32();
        let mut hi = mul_high_u32(r, n);
        let mut lo = r.wrapping_mul(n);
        if lo < n {
            let t = n.wrapping_neg() % n;
            while lo < t {
                r = self.gen_u32();
                hi = mul_high_u32(r, n);
                lo = r.wrapping_mul(n);
            }
        }
        hi
    }

    /// Generates a random `u64` in `0..n`.
    #[inline]
    fn gen_mod_u64(&self, n: u64) -> u64 {
        // Adapted from: https://lemire.me/blog/2016/06/30/fast-random-shuffling/
        let mut r = self.gen_u64();
        let mut hi = mul_high_u64(r, n);
        let mut lo = r.wrapping_mul(n);
        if lo < n {
            let t = n.wrapping_neg() % n;
            while lo < t {
                r = self.gen_u64();
                hi = mul_high_u64(r, n);
                lo = r.wrapping_mul(n);
            }
        }
        hi
    }

    /// Generates a random `u128` in `0..n`.
    #[inline]
    fn gen_mod_u128(&self, n: u128) -> u128 {
        // Adapted from: https://lemire.me/blog/2016/06/30/fast-random-shuffling/
        let mut r = self.gen_u128();
        let mut hi = mul_high_u128(r, n);
        let mut lo = r.wrapping_mul(n);
        if lo < n {
            let t = n.wrapping_neg() % n;
            while lo < t {
                r = self.gen_u128();
                hi = mul_high_u128(r, n);
                lo = r.wrapping_mul(n);
            }
        }
        hi
    }
}

/// Computes `(a * b) >> 32`.
#[inline]
fn mul_high_u32(a: u32, b: u32) -> u32 {
    (((a as u64) * (b as u64)) >> 32) as u32
}

/// Computes `(a * b) >> 64`.
#[inline]
fn mul_high_u64(a: u64, b: u64) -> u64 {
    (((a as u128) * (b as u128)) >> 64) as u64
}

/// Computes `(a * b) >> 128`.
#[inline]
fn mul_high_u128(a: u128, b: u128) -> u128 {
    // Adapted from: https://stackoverflow.com/a/28904636
    let a_lo = a as u64 as u128;
    let a_hi = (a >> 64) as u64 as u128;
    let b_lo = b as u64 as u128;
    let b_hi = (b >> 64) as u64 as u128;
    let carry = (a_lo * b_lo) >> 64;
    let carry = ((a_hi * b_lo) as u64 as u128 + (a_lo * b_hi) as u64 as u128 + carry) >> 64;
    a_hi * b_hi + ((a_hi * b_lo) >> 64) + ((a_lo * b_hi) >> 64) + carry
}

macro_rules! rng_integer {
    ($t:tt, $unsigned_t:tt, $gen:tt, $mod:tt, $doc:tt) => {
        #[doc = $doc]
        ///
        /// Panics if the range is empty.
        #[inline]
        pub fn $t(&self, range: impl RangeBounds<$t>) -> $t {
            let panic_empty_range = || {
                panic!(
                    "empty range: {:?}..{:?}",
                    range.start_bound(),
                    range.end_bound()
                )
            };

            let low = match range.start_bound() {
                Bound::Unbounded => std::$t::MIN,
                Bound::Included(&x) => x,
                Bound::Excluded(&x) => x.checked_add(1).unwrap_or_else(panic_empty_range),
            };

            let high = match range.end_bound() {
                Bound::Unbounded => std::$t::MAX,
                Bound::Included(&x) => x,
                Bound::Excluded(&x) => x.checked_sub(1).unwrap_or_else(panic_empty_range),
            };

            if low > high {
                panic_empty_range();
            }

            if low == std::$t::MIN && high == std::$t::MAX {
                self.$gen() as $t
            } else {
                let len = high.wrapping_sub(low).wrapping_add(1);
                low.wrapping_add(self.$mod(len as $unsigned_t as _) as $t)
            }
        }
    };
}

impl Rng {
    /// Creates a new random number generator.
    #[inline]
    pub fn new() -> Rng {
        Rng::with_seed(
            RNG.try_with(|rng| rng.u64(..))
                .unwrap_or(0x4d595df4d0f33173),
        )
    }

    /// Creates a new random number generator with the initial seed.
    #[inline]
    pub fn with_seed(seed: u64) -> Self {
        let rng = Rng(Cell::new(0));
        rng.seed(seed);
        rng
    }

    /// Initializes this generator with the given seed.
    #[inline]
    pub fn seed(&self, seed: u64) {
        self.0.set(seed);
    }

    /// Gives back **current** seed that is being held by this generator.
    #[inline]
    pub fn get_seed(&self) -> u64 {
        self.0.get()
    }

    /// Collects amount values at random from the iterator into a vector.
    #[inline]
    pub fn choose_multiple<T: Iterator + Sized>(&self, mut iter: T, amount: usize) -> Vec<T::Item> {
        // Adapted from: https://docs.rs/rand/latest/rand/seq/trait.IteratorRandom.html#method.choose_multiple
        let mut reservoir = Vec::with_capacity(amount);
        reservoir.extend(iter.by_ref().take(amount));
        if reservoir.len() == amount {
            for (i, elem) in iter.enumerate() {
                let end = i + 1 + amount;
                let k = self.usize(..end);
                if let Some(slot) = reservoir.get_mut(k) {
                    *slot = elem;
                }
            }
        } else {
            reservoir.shrink_to_fit();
        }
        reservoir
    }

    /// Generates a random `f64` in range `0..1`.
    pub fn f64(&self) -> f64 {
        let b = 64;
        let f = std::f64::MANTISSA_DIGITS - 1;
        f64::from_bits((1 << (b - 2)) - (1 << f) + (self.u64(..) >> (b - f))) - 1.0
    }

    rng_integer!(
        u8,
        u8,
        gen_u32,
        gen_mod_u32,
        "Generates a random `u8` in the given range."
    );

    rng_integer!(
        u64,
        u64,
        gen_u64,
        gen_mod_u64,
        "Generates a random `u64` in the given range."
    );

    #[cfg(target_pointer_width = "16")]
    rng_integer!(
        usize,
        usize,
        gen_u32,
        gen_mod_u32,
        "Generates a random `usize` in the given range."
    );
    #[cfg(target_pointer_width = "32")]
    rng_integer!(
        usize,
        usize,
        gen_u32,
        gen_mod_u32,
        "Generates a random `usize` in the given range."
    );
    #[cfg(target_pointer_width = "64")]
    rng_integer!(
        usize,
        usize,
        gen_u64,
        gen_mod_u64,
        "Generates a random `usize` in the given range."
    );
    #[cfg(target_pointer_width = "128")]
    rng_integer!(
        usize,
        usize,
        gen_u128,
        gen_mod_u128,
        "Generates a random `usize` in the given range."
    );
}

thread_local! {
    static RNG: Rng = Rng(Cell::new({
        let mut hasher = DefaultHasher::new();
        Instant::now().hash(&mut hasher);
        thread::current().id().hash(&mut hasher);
        let hash = hasher.finish();
        (hash << 1) | 1
    }));
}

/// Collects `amount` values at random from the iterator into a vector.
#[inline]
pub fn choose_multiple<T: Iterator + Sized>(iter: T, amount: usize) -> Vec<T::Item> {
    RNG.with(|rng| rng.choose_multiple(iter, amount))
}

/// Generates a random `f64` in range `0..1`.
pub fn f64() -> f64 {
    RNG.with(|rng| rng.f64())
}

macro_rules! integer {
    ($t:tt, $doc:tt) => {
        #[doc = $doc]
        ///
        /// Panics if the range is empty.
        #[inline]
        pub fn $t(range: impl RangeBounds<$t>) -> $t {
            RNG.with(|rng| rng.$t(range))
        }
    };
}

integer!(u8, "Generates a random `u8` in the given range.");
integer!(usize, "Generates a random `usize` in the given range.");
