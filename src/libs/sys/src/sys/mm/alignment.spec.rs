verus! {

// Abstract numeric value (in bytes) of an [`Alignment`]. Mirrors the enum's
// discriminants so specs can relate `is_aligned` results to the underlying modulus.
pub open spec fn spec_align_value(align: Alignment) -> int {
    match align {
        Alignment::Align4 => 4,
        Alignment::Align8 => 8,
        Alignment::Align16 => 16,
        Alignment::Align32 => 32,
        Alignment::Align64 => 64,
        Alignment::Align128 => 128,
        Alignment::Align256 => 256,
        Alignment::Align512 => 512,
        Alignment::Align1024 => 1024,
        Alignment::Align2048 => 2048,
        Alignment::Align4096 => 4096,
        Alignment::Align8192 => 8192,
        Alignment::Align16384 => 16384,
        Alignment::Align32768 => 32768,
        Alignment::Align65536 => 65536,
        Alignment::Align131072 => 131072,
        Alignment::Align262144 => 262144,
        Alignment::Align524288 => 524288,
        Alignment::Align1048576 => 1048576,
        Alignment::Align2097152 => 2097152,
        Alignment::Align4194304 => 4194304,
    }
}

} // verus!
