// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;

// Known SysV hash values (hand-computed).
#[test]
fn elf_hash_reference_values() {
    assert_eq!(elf_hash(b""), 0);
    assert_eq!(elf_hash(b"a"), 0x61);
    assert_eq!(elf_hash(b"ab"), 0x672);
}

// Known GNU hash values (seed 5381, djb2).
#[test]
fn gnu_hash_reference_values() {
    assert_eq!(gnu_hash(b""), 0x1505);
    assert_eq!(gnu_hash(b"a"), 177670);
}

/// Builds a `.hash` section (as bytes) for a set of named symbols placed at
/// indices `1..=names.len()` (index 0 is the reserved null symbol).
fn build_sysv(names: &[&str], nbucket: u32) -> Vec<u8> {
    let nchain: u32 = names.len() as u32 + 1;
    let mut bucket: Vec<u32> = vec![STN_UNDEF; nbucket as usize];
    let mut chain: Vec<u32> = vec![STN_UNDEF; nchain as usize];

    // Insert each symbol at the head of its bucket's chain.
    for (i, name) in names.iter().enumerate() {
        let sym_idx: u32 = i as u32 + 1;
        let b: usize = (elf_hash(name.as_bytes()) % nbucket) as usize;
        chain[sym_idx as usize] = bucket[b];
        bucket[b] = sym_idx;
    }

    let mut out: Vec<u8> = Vec::new();
    out.extend_from_slice(&nbucket.to_le_bytes());
    out.extend_from_slice(&nchain.to_le_bytes());
    for w in bucket.iter().chain(chain.iter()) {
        out.extend_from_slice(&w.to_le_bytes());
    }
    out
}

#[test]
fn sysv_lookup_present_and_absent() {
    let names: [&str; 5] = ["printf", "malloc", "free", "memcpy", "strlen"];
    let bytes: Vec<u8> = build_sysv(&names, 3);
    // SAFETY: `bytes` outlives the table and is large enough by construction.
    let table: SysvHashTable =
        unsafe { SysvHashTable::from_raw_parts(bytes.as_ptr(), bytes.len()) }
            .expect("well-formed SysV hash table should parse");

    let symtab_len: usize = names.len() + 1;
    for (i, name) in names.iter().enumerate() {
        let found: Lookup = table.lookup(name.as_bytes(), symtab_len, |idx| idx as usize == i + 1);
        assert_eq!(found, Lookup::Found(i as u32 + 1), "missing {name}");
    }

    // Absent name: the collision predicate never fires, so lookup fails.
    assert_eq!(table.lookup(b"nonexistent", symtab_len, |_| false), Lookup::NotFound);
    // A name that shares a bucket but whose predicate rejects every
    // candidate must still fail.
    assert_eq!(table.lookup(b"printf", symtab_len, |_| false), Lookup::NotFound);
}

/// Builds a `.gnu.hash` section (as bytes) for a set of named symbols. The
/// symbols are sorted by bucket and placed at indices `symoffset..`.
/// Returns `(bytes, ordered_names, symoffset)`.
fn build_gnu<'a>(names: &[&'a str], nbuckets: u32, symoffset: u32) -> (Vec<u8>, Vec<&'a str>, u32) {
    let bloom_size: u32 = 1;
    let bloom_shift: u32 = 6;

    // Order symbols by bucket so each bucket owns a contiguous index range.
    let mut ordered: Vec<&str> = names.to_vec();
    ordered.sort_by_key(|n| gnu_hash(n.as_bytes()) % nbuckets);

    let count: usize = ordered.len();
    let mut bloom: Vec<ElfBloomWord> = vec![0; bloom_size as usize];
    let mut buckets: Vec<u32> = vec![STN_UNDEF; nbuckets as usize];
    let mut chain: Vec<u32> = vec![0; count];

    for (pos, name) in ordered.iter().enumerate() {
        let sym_idx: u32 = symoffset + pos as u32;
        let h: u32 = gnu_hash(name.as_bytes());
        let b: u32 = h % nbuckets;

        // Bloom filter bits.
        let word: usize = ((h / ELFCLASS_BITS) % bloom_size) as usize;
        bloom[word] |= ((1 as ElfBloomWord) << (h % ELFCLASS_BITS))
            | ((1 as ElfBloomWord) << ((h >> bloom_shift) % ELFCLASS_BITS));

        // First symbol of a bucket records its start index.
        if buckets[b as usize] == STN_UNDEF {
            buckets[b as usize] = sym_idx;
        }

        // Chain stores the hash; the low bit is the end-of-chain flag, set
        // when the next symbol falls in a different bucket (or is the last).
        let mut val: u32 = h & !1;
        let last_in_bucket: bool =
            pos + 1 == count || gnu_hash(ordered[pos + 1].as_bytes()) % nbuckets != b;
        if last_in_bucket {
            val |= 1;
        }
        chain[pos] = val;
    }

    let mut out: Vec<u8> = Vec::new();
    out.extend_from_slice(&nbuckets.to_le_bytes());
    out.extend_from_slice(&symoffset.to_le_bytes());
    out.extend_from_slice(&bloom_size.to_le_bytes());
    out.extend_from_slice(&bloom_shift.to_le_bytes());
    for w in bloom {
        out.extend_from_slice(&w.to_le_bytes());
    }
    for w in buckets.iter().chain(chain.iter()) {
        out.extend_from_slice(&w.to_le_bytes());
    }
    (out, ordered, symoffset)
}

#[test]
fn gnu_lookup_present_and_absent() {
    let names: [&str; 6] = ["printf", "malloc", "free", "memcpy", "strlen", "cos"];
    let symoffset: u32 = 1;
    let (bytes, ordered, off) = build_gnu(&names, 2, symoffset);
    let symtab_len: usize = off as usize + ordered.len();

    // SAFETY: `bytes` outlives the table and is large enough by construction.
    let table: GnuHashTable =
        unsafe { GnuHashTable::from_raw_parts(bytes.as_ptr(), bytes.len(), symtab_len) }
            .expect("well-formed GNU hash table should parse");

    for (pos, name) in ordered.iter().enumerate() {
        let expect: u32 = off + pos as u32;
        let found: Lookup = table.lookup(name.as_bytes(), symtab_len, |idx| idx == expect);
        assert_eq!(found, Lookup::Found(expect), "missing {name}");
    }

    // Absent name: rejected by the Bloom filter or an empty/short chain.
    assert_eq!(table.lookup(b"nonexistent_symbol", symtab_len, |_| false), Lookup::NotFound);
    // Present hash but rejected predicate must still fail.
    assert_eq!(table.lookup(b"printf", symtab_len, |_| false), Lookup::NotFound);
}

#[test]
fn rejects_invalid_gnu_bloom_shift() {
    let (mut bytes, ordered, symoffset) = build_gnu(&["symbol"], 1, 1);
    bytes[12..16].copy_from_slice(&u32::BITS.to_le_bytes());
    let symtab_len: usize = symoffset as usize + ordered.len();

    // SAFETY: `bytes` outlives the parser and contains a complete GNU hash table.
    assert!(
        unsafe { GnuHashTable::from_raw_parts(bytes.as_ptr(), bytes.len(), symtab_len) }.is_none()
    );
}

#[test]
fn rejects_truncated_tables() {
    // SysV: the header claims four buckets and four chains, but no bucket or
    // chain payload follows, so the region is too small.
    let mut sysv: Vec<u8> = Vec::new();
    sysv.extend_from_slice(&4u32.to_le_bytes()); // nbucket
    sysv.extend_from_slice(&4u32.to_le_bytes()); // nchain
                                                 // SAFETY: pointer/len describe a real (too-small) allocation.
    assert!(unsafe { SysvHashTable::from_raw_parts(sysv.as_ptr(), sysv.len()) }.is_none());

    // GNU: a header that is too short is rejected.
    let gnu: [u8; 8] = [0; 8];
    // SAFETY: pointer/len describe a real (too-small) allocation.
    assert!(unsafe { GnuHashTable::from_raw_parts(gnu.as_ptr(), gnu.len(), 4) }.is_none());
}
