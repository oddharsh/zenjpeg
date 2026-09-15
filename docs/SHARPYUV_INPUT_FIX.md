# SharpYUV input interpretation fix for v0.8.4

The encoder's gamma-aware strip path passed only bytes per pixel to helpers
that read the first three bytes of each pixel as RGB8. With linear f32/u16
layouts those were sample representation bytes; with BGR they were reversed
channels. Both gamma-aware methods were affected when chroma was subsampled.
The regular 4:4:4 path was unaffected.

The strip path now passes the actual pixel format and row stride. A shared
reader interprets RGB/RGBA f32/u16 as linear sRGB, preserves fractional samples,
and reads byte RGB/BGR in channel order. It feeds the existing gamma-aware and
iterative algorithms without an RGB8 intermediate or additional image/strip
allocation. The exported byte helper signatures and their RGB8 interpretation
are preserved. Alpha remains ignored; float layouts retain their SDR [0, 1]
contract.

## Regression evidence

`tests/sharp_yuv_input_formats.rs` uses generated coloured patches with fine
texture and the independent `jpeg-decoder` crate. It covers both gamma methods,
RGB/RGBA f32/u16 and RGB/RGBA/BGR/BGRA8, 4:4:4/4:2:2/4:2:0/4:4:0, one-pixel
edges, odd sizes, padded rows and seven-row pushes. Equivalent layouts must
remain within 0.8 mean and 6 maximum decoded channel steps of RGB8; padding
and chunking must produce identical JPEG bytes. A fractional-sample test rejects
an intermediate eight-bit rounding pass.

On unmodified v0.8.4, five of the original six tests failed. A one-pixel f32
case had a maximum channel error of 63; the u16 case reached 162 and BGR 176.
After the fix, all six pass, including the expanded two-method matrix.

## Validation (macOS arm64, Rust 1.93.0)

```sh
git submodule update --init --recursive internal/jpegli-cpp
cargo test --release -p zenjpeg --lib --features trellis --locked --offline
cargo test --release -p zenjpeg --test sharp_yuv_input_formats --features trellis --locked --offline
cargo fmt -p zenjpeg -- --check
cargo clippy -p zenjpeg -- -D warnings
```

- Library tests with trellis: **881 passed**. Unmodified release without
  trellis was also checked before the fix: **782 passed**.
- Regression tests: **6 passed**. Formatting: passed.
- Clippy: **failed on six existing errors in unchanged files**: two unknown
  `manual_checked_ops` lint declarations, duplicate aarch64 configuration,
  unused `SimdToken`, unused `w`, and an unnecessary unwrap. These were not
  suppressed or changed by this fix.
- Test builds also retain upstream unused-manifest/import/helper warnings and
  C++ reference-wrapper recursion warnings. The external C++ corpus comparison
  suite was not run; regression decoding uses the independent Rust decoder.

A separate before/after compatibility run compared the published 0.8.4 crate
against this source on generated 1440x960 RGB8/RGBA8 images at q99 with hybrid
trellis and progressive scan search. All seven layout/subsampling cases were
byte-identical. With five alternating timed runs after warm-up, subsampled
encoding took 1.7–5.5% longer (roughly 7–20 ms/image); 4:4:4 timing was unchanged.
These local timings describe this sample and machine, not a general speed bound.
