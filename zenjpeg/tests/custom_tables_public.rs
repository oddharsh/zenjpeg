//! `QuantTableConfig::Custom` is reachable through the public API, and
//! `ScalingParams::Exact` writes the given tables to DQT verbatim.

use zenjpeg::encoder::{
    ChromaSubsampling, EncoderConfig, EncodingTables, PerComponent, PixelLayout, QuantTableConfig,
    ScalingParams, Unstoppable,
};

const ZIGZAG: [usize; 64] = [
    0, 1, 8, 16, 9, 2, 3, 10, 17, 24, 32, 25, 18, 11, 4, 5, 12, 19, 26, 33, 40, 48, 41, 34, 27, 20,
    13, 6, 7, 14, 21, 28, 35, 42, 49, 56, 57, 50, 43, 36, 29, 22, 15, 23, 30, 37, 44, 51, 58, 59,
    52, 45, 38, 31, 39, 46, 53, 60, 61, 54, 47, 55, 62, 63,
];

/// Every 8-bit DQT table in the file, in natural order, keyed by slot.
fn dqt_tables(bytes: &[u8]) -> Vec<(u8, [u16; 64])> {
    let mut out = Vec::new();
    let mut i = 2;
    while i + 4 <= bytes.len() && bytes[i] == 0xff && bytes[i + 1] != 0xda {
        let len = usize::from(bytes[i + 2]) << 8 | usize::from(bytes[i + 3]);
        if bytes[i + 1] == 0xdb {
            let seg = &bytes[i + 4..i + 2 + len];
            let mut p = 0;
            while p + 65 <= seg.len() {
                assert_eq!(seg[p] >> 4, 0, "8-bit tables expected");
                let mut natural = [0u16; 64];
                for (k, &v) in seg[p + 1..p + 65].iter().enumerate() {
                    natural[ZIGZAG[k]] = u16::from(v);
                }
                out.push((seg[p] & 15, natural));
                p += 65;
            }
        }
        i += 2 + len;
    }
    out
}

#[test]
fn exact_custom_tables_are_written_verbatim() {
    let luma: [f32; 64] = std::array::from_fn(|k| (2 + (k % 8 + k / 8)) as f32);
    let chroma: [f32; 64] = std::array::from_fn(|k| (3 + 2 * (k % 8 + k / 8)).min(40) as f32);
    let mut tables = EncodingTables::default_ycbcr();
    tables.quant = PerComponent {
        c0: luma,
        c1: chroma,
        c2: chroma,
    };
    tables.scaling = ScalingParams::Exact;

    let (w, h) = (24u32, 16u32);
    let px: Vec<f32> = (0..w * h)
        .flat_map(|i| {
            let (x, y) = ((i % w) as f32 / w as f32, (i / w) as f32 / h as f32);
            [x * x, 0.2, y]
        })
        .collect();
    let mut enc = EncoderConfig::ycbcr(90, ChromaSubsampling::None)
        .quant_table_config(QuantTableConfig::Custom(Box::new(tables)))
        .request()
        .encode_from_bytes(w, h, PixelLayout::RgbF32Linear)
        .unwrap();
    enc.push_packed(bytemuck::cast_slice(&px), Unstoppable)
        .unwrap();
    let bytes = enc.finish().unwrap();

    let found = dqt_tables(&bytes);
    let want = |t: &[f32; 64]| t.map(|v| v as u16);
    assert_eq!(found.len(), 3, "separate Cb and Cr tables");
    assert_eq!(found[0], (0, want(&luma)));
    assert_eq!(found[1], (1, want(&chroma)));
    assert_eq!(found[2], (2, want(&chroma)));
}
