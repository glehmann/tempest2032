//! The original 256-entry signed sine table from `dat/sines.dat`, loaded from
//! the `sines` label in `src/yak.s`.
//!
//! This is a hand-authored table, not a computed sine. 78 of its 256 entries
//! sit one unit away from `trunc(127 * sin(2*pi*i/256))`, the peak is flat at
//! 126 across indices 59 to 69 where a real sine reaches 127, and it is not
//! half-wave symmetric: index 5 is 12 while index 133 is -16. The call sites
//! feed these values into integer shifts and CRY color masks, so a generated
//! table would move geometry. Keep the numbers as they are.

#[rustfmt::skip]
pub(crate) const SINES: [i8; 256] = [
       0,    3,    6,    9,   12,   15,   18,   21,   24,   27,   30,   33,   36,   39,   42,   45,
      48,   51,   54,   57,   59,   62,   65,   68,   70,   73,   75,   78,   80,   83,   85,   87,
      89,   92,   94,   96,   98,  100,  102,  103,  105,  107,  109,  110,  112,  113,  114,  116,
     117,  118,  119,  120,  121,  122,  123,  123,  124,  125,  125,  126,  126,  126,  126,  126,
     126,  126,  126,  126,  126,  126,  125,  125,  124,  123,  123,  122,  121,  120,  119,  118,
     117,  115,  114,  113,  111,  110,  108,  107,  105,  103,  101,   99,   97,   95,   93,   91,
      89,   87,   85,   82,   80,   77,   75,   72,   70,   67,   64,   62,   59,   56,   53,   51,
      48,   45,   42,   39,   36,   33,   30,   27,   24,   21,   18,   15,   12,    8,    5,    2,
       0,   -3,   -6,   -9,  -12,  -16,  -19,  -22,  -25,  -28,  -31,  -34,  -37,  -40,  -43,  -46,
     -49,  -51,  -54,  -57,  -60,  -63,  -65,  -68,  -71,  -73,  -76,  -78,  -80,  -83,  -85,  -87,
     -90,  -92,  -94,  -96,  -98, -100, -102, -104, -105, -107, -109, -110, -112, -113, -115, -116,
    -117, -118, -119, -120, -121, -122, -123, -124, -124, -125, -125, -126, -126, -126, -126, -126,
    -126, -126, -126, -126, -126, -125, -125, -125, -124, -123, -123, -122, -121, -120, -119, -118,
    -117, -115, -114, -113, -111, -110, -108, -106, -105, -103, -101,  -99,  -97,  -95,  -93,  -91,
     -89,  -86,  -84,  -82,  -79,  -77,  -74,  -72,  -69,  -67,  -64,  -61,  -59,  -56,  -53,  -50,
     -47,  -44,  -41,  -39,  -36,  -33,  -30,  -26,  -23,  -20,  -17,  -14,  -11,   -8,   -5,   -2,
];

/// Reads the table the way `move.b 0(a1,d0.w)` does, folding the angle into a
/// byte offset. `warp/tunnel.rs` passes negative angles such as `angle - 1`, so
/// the fold has to wrap rather than truncate.
pub(crate) fn sine(angle: i32) -> i32 {
    SINES[angle.rem_euclid(256) as usize] as i32
}

#[cfg(test)]
mod tests {
    use super::{SINES, sine};

    #[test]
    fn table_matches_the_original_hand_authored_values() {
        assert_eq!(sine(0), 0);
        assert_eq!(sine(64), 126);
        assert_eq!(sine(128), 0);
        assert_eq!(sine(192), -126);
        assert_eq!(
            SINES[59..=69],
            [126; 11],
            "the original peak is flat, not 127"
        );
    }

    #[test]
    fn table_is_not_a_computed_sine() {
        assert_eq!(sine(5), 15);
        assert_eq!(
            sine(133),
            -16,
            "half-wave symmetry would give -16 to be -15"
        );
        assert!(
            (0..256).any(|angle| {
                let computed = (127.0 * (angle as f32 * std::f32::consts::TAU / 256.).sin()) as i32;
                computed != sine(angle)
            }),
            "a generated sine would change geometry"
        );
    }

    #[test]
    fn negative_and_wrapped_angles_fold_into_the_table() {
        assert_eq!(sine(-1), -2, "tunnel geometry subtracts from the angle");
        assert_eq!(sine(-256), 0);
        assert_eq!(sine(255), -2);
        assert_eq!(sine(256), 0);
    }
}
