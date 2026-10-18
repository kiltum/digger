//! The eight original level layouts from `main.c` (VGA build).
//!
//! Legend: ` ` solid dirt, `H` horizontal tunnel, `V` vertical tunnel,
//! `C` emerald, `B` bag, `S` the digger's start cell (a vertical tunnel).

/// `levplan()`: levels 1..8 use plans 1..8; beyond that the original cycles
/// `6,7,8,5` (`(l & 3) + 5`) forever.
pub fn level_plan(level: i32) -> i32 {
    if level > 8 {
        (level & 3) + 5
    } else {
        level.max(1)
    }
}

pub const LEVELS: [[&[u8; 15]; 10]; 8] = [
    [
        b"S   B     HHHHS",
        b"V  CC  C  V B  ",
        b"VB CC  C  V    ",
        b"V  CCB CB V CCC",
        b"V  CC  C  V CCC",
        b"HH CC  C  V CCC",
        b" V    B B V    ",
        b" HHHH     V    ",
        b"C   V     V   C",
        b"CC  HHHHHHH  CC",
    ],
    [
        b"SHHHHH  B B  HS",
        b" CC  V       V ",
        b" CC  V CCCCC V ",
        b"BCCB V CCCCC V ",
        b"CCCC V       V ",
        b"CCCC V B  HHHH ",
        b" CC  V CC V    ",
        b" BB  VCCCCV CC ",
        b"C    V CC V CC ",
        b"CC   HHHHHH    ",
    ],
    [
        b"SHHHHB B BHHHHS",
        b"CC  V C C V BB ",
        b"C   V C C V CC ",
        b" BB V C C VCCCC",
        b"CCCCV C C VCCCC",
        b"CCCCHHHHHHH CC ",
        b" CC  C V C  CC ",
        b" CC  C V C     ",
        b"C    C V C    C",
        b"CC   C H C   CC",
    ],
    [
        b"SHBCCCCBCCCCBHS",
        b"CV  CCCCCCC  VC",
        b"CHHH CCCCC HHHC",
        b"C  V  CCC  V  C",
        b"   HHH C HHH   ",
        b"  B  V B V  B  ",
        b"  C  VCCCV  C  ",
        b" CCC HHHHH CCC ",
        b"CCCCC CVC CCCCC",
        b"CCCCC CHC CCCCC",
    ],
    [
        b"SHHHHHHHHHHHHHS",
        b"VBCCCCBVCCCCCCV",
        b"VCCCCCCV CCBC V",
        b"V CCCC VCCBCCCV",
        b"VCCCCCCV CCCC V",
        b"V CCCC VBCCCCCV",
        b"VCCBCCCV CCCC V",
        b"V CCBC VCCCCCCV",
        b"VCCCCCCVCCCCCCV",
        b"HHHHHHHHHHHHHHH",
    ],
    [
        b"SHHHHHHHHHHHHHS",
        b"VCBCCV V VCCBCV",
        b"VCCC VBVBV CCCV",
        b"VCCCHH V HHCCCV",
        b"VCC V CVC V CCV",
        b"VCCHH CVC HHCCV",
        b"VC V CCVCC V CV",
        b"VCHHBCCVCCBHHCV",
        b"VCVCCCCVCCCCVCV",
        b"HHHHHHHHHHHHHHH",
    ],
    [
        b"SHCCCCCVCCCCCHS",
        b" VCBCBCVCBCBCV ",
        b"BVCCCCCVCCCCCVB",
        b"CHHCCCCVCCCCHHC",
        b"CCV CCCVCCC VCC",
        b"CCHHHCCVCCHHHCC",
        b"CCCCV CVC VCCCC",
        b"CCCCHH V HHCCCC",
        b"CCCCCV V VCCCCC",
        b"CCCCCHHHHHCCCCC",
    ],
    [
        b"HHHHHHHHHHHHHHS",
        b"V CCBCCCCCBCC V",
        b"HHHCCCCBCCCCHHH",
        b"VBV CCCCCCC VBV",
        b"VCHHHCCCCCHHHCV",
        b"VCCBV CCC VBCCV",
        b"VCCCHHHCHHHCCCV",
        b"VCCCC V V CCCCV",
        b"VCCCCCV VCCCCCV",
        b"HHHHHHHHHHHHHHH",
    ],
];

#[cfg(test)]
mod tests {
    use super::LEVELS;

    #[test]
    fn every_row_is_fifteen_cells() {
        for (l, level) in LEVELS.iter().enumerate() {
            for (y, row) in level.iter().enumerate() {
                assert_eq!(row.len(), 15, "level {} row {}", l + 1, y);
            }
            assert_eq!(level.len(), 10, "level {} row count", l + 1);
        }
    }

    #[test]
    fn characters_are_from_the_original_alphabet() {
        for level in LEVELS.iter() {
            for row in level.iter() {
                for &c in row.iter() {
                    assert!(
                        matches!(c, b' ' | b'H' | b'V' | b'C' | b'B' | b'S'),
                        "unexpected level character {:?}",
                        c as char
                    );
                }
            }
        }
    }

    #[test]
    fn level_plan_sequence() {
        use super::level_plan;
        let first: Vec<i32> = (1..=8).map(level_plan).collect();
        assert_eq!(first, vec![1, 2, 3, 4, 5, 6, 7, 8]);
        let after: Vec<i32> = (9..=16).map(level_plan).collect();
        assert_eq!(after, vec![6, 7, 8, 5, 6, 7, 8, 5]);
        assert_eq!(level_plan(1000), (1000 & 3) + 5);
    }

    #[test]
    fn every_level_has_a_start_cell() {
        for (l, level) in LEVELS.iter().enumerate() {
            assert!(
                level.iter().any(|row| row.contains(&b'S')),
                "level {} has no S",
                l + 1
            );
        }
    }
}
