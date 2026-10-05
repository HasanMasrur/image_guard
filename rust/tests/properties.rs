//! Property tests: random sizes/options must never panic and must always
//! honour the "≤ max_bytes or a clear error" contract.

mod common;

use common::*;
use image_guard::api::compress::compress_bytes;
use image_guard::api::types::*;
use proptest::prelude::*;

proptest! {
    #![proptest_config(ProptestConfig { cases: 48, ..ProptestConfig::default() })]

    #[test]
    fn contract_holds(
        w in 1u32..700,
        h in 1u32..700,
        max_kb in 1u32..120,
        min_q in 1u8..=100,
        span in 0u8..=100,
        min_dim in 1u32..300,
        max_w in proptest::option::of(1u32..900),
        png_out in any::<bool>(),
        noisy in any::<bool>(),
    ) {
        let max_q = min_q.saturating_add(span).min(100);
        let img = if noisy { noise(w, h) } else { photo(w, h) };
        let input = jpeg(&img, 90);
        let o = CompressOptions {
            max_bytes: max_kb * KB,
            max_width: max_w,
            min_quality: min_q,
            max_quality: max_q,
            min_dimension: min_dim,
            format: if png_out { OutputFormat::Png } else { OutputFormat::Jpeg },
            ..Default::default()
        };
        match compress_bytes(input, o.clone()) {
            Ok(r) => {
                prop_assert!(r.size_bytes <= o.max_bytes);
                prop_assert!(r.width <= w.max(1) && r.height <= h.max(1));
                if let Some(mw) = max_w { prop_assert!(r.width <= mw); }
                decode(&r.bytes);
            }
            Err(e) => prop_assert!(
                matches!(e.code, ErrorCode::InvalidOptions | ErrorCode::CannotMeetTarget),
                "unexpected error {:?}", e
            ),
        }
    }

    /// Random byte corruption must give an error or a valid image — never a panic.
    #[test]
    fn corrupted_input_never_panics(flips in proptest::collection::vec((0usize..4000, any::<u8>()), 1..20)) {
        let mut input = jpeg(&photo(80, 60), 85);
        for (pos, val) in flips {
            let i = pos % input.len();
            input[i] = val;
        }
        if let Ok(r) = compress_bytes(input, CompressOptions { max_bytes: 20 * KB, ..Default::default() }) {
            prop_assert!(r.size_bytes <= 20 * KB);
        }
    }

    #[test]
    fn random_bytes_never_panic(bytes in proptest::collection::vec(any::<u8>(), 0..2048)) {
        let _ = compress_bytes(bytes, CompressOptions::default());
    }
}
