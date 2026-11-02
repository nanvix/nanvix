// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//!
//! # ELF Symbol Hash Tables
//!
//! Accelerated symbol lookup for the dynamic loader via the two standard ELF
//! symbol hash tables:
//!
//! - `DT_HASH` (`.hash`) - the classic System V ELF hash table.
//! - `DT_GNU_HASH` (`.gnu.hash`) - the GNU-extension hash table, prefixed with a
//!   Bloom filter that rejects most absent symbols without walking a chain.
//!
//! Both replace a linear scan of `.dynsym` (`O(N)` string comparisons per
//! lookup) with a hash-bucket walk whose typical chain length is one to three
//! entries, making lookups effectively `O(1)`.
//!
//! The parsers validate the on-disk layout up front and return [`None`] on any
//! inconsistency, so a caller that cannot build an accelerator can safely fall
//! back to a linear scan while still producing correct results.
//!
//! # ELF class
//!
//! SysV hash-table words and GNU hash bucket/chain words are always `u32`.
//! GNU Bloom-filter words follow the ELF class: `u32` for ELFCLASS32 and `u64`
//! for ELFCLASS64.
//!

//==================================================================================================
// Imports
//==================================================================================================

use ::core::{
    mem,
    ptr,
};

//==================================================================================================
// Constants
//==================================================================================================

/// Reserved symbol-table index denoting "no symbol" (`STN_UNDEF`). A SysV hash
/// bucket or chain slot holding this value terminates the chain, and a GNU hash
/// bucket holding it denotes an empty bucket.
const STN_UNDEF: u32 = 0;

/// GNU hash Bloom-filter word for the active guest ELF class.
#[cfg(not(target_arch = "x86_64"))]
type ElfBloomWord = u32;
/// GNU hash Bloom-filter word for the active guest ELF class.
#[cfg(target_arch = "x86_64")]
type ElfBloomWord = u64;

/// Number of bits in a GNU hash Bloom-filter word for the active ELF class.
const ELFCLASS_BITS: u32 = (mem::size_of::<ElfBloomWord>() * 8) as u32;
/// Number of bytes in a GNU hash Bloom-filter word for the active ELF class.
const ELFCLASS_BYTES: usize = mem::size_of::<ElfBloomWord>();

//==================================================================================================
// Hash functions
//==================================================================================================

///
/// # Description
///
/// Computes the System V ELF hash (`DT_HASH`) of a symbol name.
///
/// This is the hash function published in the System V gABI and emitted by every
/// ELF linker for the `.hash` table. It is computed with 32-bit wrapping
/// arithmetic, matching the ELFCLASS32 objects Nanvix guests use.
///
/// # Parameters
///
/// - `name`: The raw symbol-name bytes, without a terminating NUL.
///
/// # Returns
///
/// The 32-bit SysV hash of `name`.
///
pub fn elf_hash(name: &[u8]) -> u32 {
    let mut h: u32 = 0;
    for &byte in name {
        h = (h << 4).wrapping_add(byte as u32);
        let g: u32 = h & 0xf000_0000;
        if g != 0 {
            h ^= g >> 24;
        }
        h &= !g;
    }
    h
}

///
/// # Description
///
/// Computes the GNU hash (`DT_GNU_HASH`) of a symbol name (the classic
/// `djb2`-style hash seeded with `5381`).
///
/// This matches the hash emitted by linkers for the `.gnu.hash` table. It is
/// computed with 32-bit wrapping arithmetic.
///
/// # Parameters
///
/// - `name`: The raw symbol-name bytes, without a terminating NUL.
///
/// # Returns
///
/// The 32-bit GNU hash of `name`.
///
pub fn gnu_hash(name: &[u8]) -> u32 {
    let mut h: u32 = 5381;
    for &byte in name {
        h = h.wrapping_mul(33).wrapping_add(byte as u32);
    }
    h
}

//==================================================================================================
// Raw reads
//==================================================================================================

///
/// # Description
///
/// Reads a little-endian `u32` located `off` bytes past `base`.
///
/// # Parameters
///
/// - `base`: Base pointer of the hash-table region.
/// - `off`: Byte offset of the word to read.
///
/// # Returns
///
/// The `u32` stored at `base + off`.
///
/// # Safety
///
/// The range `[base + off, base + off + 4)` must lie within a single readable
/// allocation that outlives the call. The read is unaligned, so no alignment
/// requirement is imposed on `base + off`.
///
#[inline]
unsafe fn read_u32(base: *const u8, off: usize) -> u32 {
    u32::from_le(ptr::read_unaligned(base.add(off) as *const u32))
}

///
/// # Description
///
/// Reads an ELF-class-sized GNU hash Bloom-filter word located `off` bytes past `base`.
///
/// # Safety
///
/// The range `[base + off, base + off + ELFCLASS_BYTES)` must lie within a readable allocation
/// that outlives the call.
///
#[inline]
unsafe fn read_bloom_word(base: *const u8, off: usize) -> ElfBloomWord {
    #[cfg(not(target_arch = "x86_64"))]
    {
        u32::from_le(ptr::read_unaligned(base.add(off) as *const u32))
    }
    #[cfg(target_arch = "x86_64")]
    {
        u64::from_le(ptr::read_unaligned(base.add(off) as *const u64))
    }
}

//==================================================================================================
// Lookup
//==================================================================================================

///
/// # Description
///
/// Outcome of a hash-table symbol lookup.
///
/// A lookup separates a definitive answer - the symbol is present, or it is
/// provably absent - from an inconsistency in the table itself. The parsers
/// validate a table's declared layout up front but do not walk every bucket and
/// chain entry, so a lookup can still meet an out-of-range index or a cyclic
/// chain in a malformed object. Reporting that case distinctly lets the caller
/// fall back to a linear scan instead of mistaking it for a genuine miss.
///
#[derive(Debug, PartialEq, Eq)]
pub enum Lookup {
    /// A symbol with the requested name is defined at this symbol-table index.
    Found(u32),
    /// The name is provably absent from this table.
    NotFound,
    /// The table is internally inconsistent (an out-of-range bucket or chain
    /// index, or a cyclic chain); the caller should fall back to a linear scan
    /// rather than trust this as a definitive answer.
    Inconsistent,
}

//==================================================================================================
// SysvHashTable
//==================================================================================================

///
/// # Description
///
/// A parsed System V (`.hash` / `DT_HASH`) symbol hash table.
///
/// On-disk layout (all `u32`): `nbucket`, `nchain`, then `bucket[nbucket]`
/// followed by `chain[nchain]`. `nchain` equals the number of entries in the
/// associated `.dynsym`, and both bucket and chain slots hold symbol-table
/// indices.
///
#[derive(Debug)]
pub struct SysvHashTable {
    /// Base pointer of the `.hash` section in the loaded image.
    base: *const u8,
    /// Number of hash buckets.
    nbucket: u32,
    /// Number of chain entries (equals the associated `.dynsym` length).
    nchain: u32,
}

// SAFETY: the table is an immutable view over a mapped, read-only region that
// outlives it; concurrent reads through the shared pointer are sound.
unsafe impl Send for SysvHashTable {}
unsafe impl Sync for SysvHashTable {}

impl SysvHashTable {
    ///
    /// # Description
    ///
    /// Parses a SysV hash table from a raw memory region, validating that the
    /// region is large enough to hold the header, buckets, and chains it
    /// declares.
    ///
    /// # Parameters
    ///
    /// - `base`: Pointer to the first byte of the `.hash` section.
    /// - `byte_len`: Size of the `.hash` section in bytes.
    ///
    /// # Returns
    ///
    /// A [`SysvHashTable`] on success, or [`None`] if the region is too small or
    /// declares a degenerate (zero-bucket) table.
    ///
    /// # Safety
    ///
    /// `base` must point to `byte_len` readable bytes that remain valid for the
    /// lifetime of the returned table.
    ///
    pub unsafe fn from_raw_parts(base: *const u8, byte_len: usize) -> Option<Self> {
        // Header is two u32 words: nbucket and nchain.
        if byte_len < 8 {
            return None;
        }
        let nbucket: u32 = read_u32(base, 0);
        let nchain: u32 = read_u32(base, 4);

        // A table with no buckets cannot resolve anything and would divide by
        // zero on lookup; reject it so the caller falls back to a linear scan.
        if nbucket == 0 {
            return None;
        }

        // Validate that the region holds header + bucket[] + chain[].
        let words: u64 = 2u64
            .checked_add(nbucket as u64)?
            .checked_add(nchain as u64)?;
        let total: u64 = words.checked_mul(4)?;
        if (byte_len as u64) < total {
            return None;
        }

        Some(SysvHashTable {
            base,
            nbucket,
            nchain,
        })
    }

    /// Reads `bucket[i]`. The caller must ensure `i < nbucket`.
    #[inline]
    fn bucket(&self, i: u32) -> u32 {
        // SAFETY: `from_raw_parts` validated that the region covers the whole
        // bucket array, and `i < nbucket` by the caller's contract.
        unsafe { read_u32(self.base, (2 + i as usize) * 4) }
    }

    /// Reads `chain[i]`. The caller must ensure `i < nchain`.
    #[inline]
    fn chain(&self, i: u32) -> u32 {
        // SAFETY: `from_raw_parts` validated that the region covers the whole
        // chain array, and `i < nchain` by the caller's contract.
        unsafe { read_u32(self.base, (2 + self.nbucket as usize + i as usize) * 4) }
    }

    ///
    /// # Description
    ///
    /// Looks up a symbol by name using the SysV hash chain.
    ///
    /// The chain yields candidate symbol indices sharing the name's hash bucket;
    /// `name_matches` resolves hash collisions by confirming the actual name.
    ///
    /// # Parameters
    ///
    /// - `name`: The raw symbol-name bytes to look up (without a NUL).
    /// - `symtab_len`: Number of entries in the associated symbol table.
    /// - `name_matches`: Predicate returning `true` when the symbol at the given
    ///   index has the sought name.
    ///
    /// # Returns
    ///
    /// A [`Lookup`] describing whether the symbol was found, is provably absent,
    /// or the table is inconsistent.
    ///
    pub fn lookup<F>(&self, name: &[u8], symtab_len: usize, mut name_matches: F) -> Lookup
    where
        F: FnMut(u32) -> bool,
    {
        let hash: u32 = elf_hash(name);
        let mut idx: u32 = self.bucket(hash % self.nbucket);
        // Bound the walk by the chain length so a malformed cyclic chain cannot
        // spin forever.
        let mut steps: u32 = 0;
        while idx != STN_UNDEF {
            // `idx` indexes both the symbol table (for the name check) and the
            // chain array (for the next hop); an out-of-range index means the
            // table is inconsistent.
            if idx >= self.nchain || idx as usize >= symtab_len {
                return Lookup::Inconsistent;
            }
            if name_matches(idx) {
                return Lookup::Found(idx);
            }
            idx = self.chain(idx);
            steps += 1;
            if steps > self.nchain {
                return Lookup::Inconsistent;
            }
        }
        Lookup::NotFound
    }
}

//==================================================================================================
// GnuHashTable
//==================================================================================================

///
/// # Description
///
/// A parsed GNU (`.gnu.hash` / `DT_GNU_HASH`) symbol hash table.
///
/// On-disk layout: a four-word header (`nbuckets`, `symoffset`, `bloom_size`,
/// `bloom_shift`), a Bloom filter of `bloom_size` ELFCLASS words, `nbuckets`
/// bucket words, and a chain array covering symbol indices `symoffset..symtab_len`.
///
/// Only symbols at index `>= symoffset` are hashed; the leading `symoffset`
/// entries (the null symbol plus any undefined/local symbols) are not present in
/// the table. A lookup for such a symbol therefore returns [`None`], which is the
/// correct answer for the loader: an undefined symbol is never *defined* by this
/// object, so resolution proceeds to the next scope regardless.
///
#[derive(Debug)]
pub struct GnuHashTable {
    /// Base pointer of the `.gnu.hash` section in the loaded image.
    base: *const u8,
    /// Number of hash buckets.
    nbuckets: u32,
    /// Index of the first hashed symbol in the associated `.dynsym`.
    symoffset: u32,
    /// Number of ELFCLASS words in the Bloom filter.
    bloom_size: u32,
    /// Right-shift applied to the hash for the Bloom filter's second bit.
    bloom_shift: u32,
}

// SAFETY: the table is an immutable view over a mapped, read-only region that
// outlives it; concurrent reads through the shared pointer are sound.
unsafe impl Send for GnuHashTable {}
unsafe impl Sync for GnuHashTable {}

impl GnuHashTable {
    ///
    /// # Description
    ///
    /// Parses a GNU hash table from a raw memory region, validating that the
    /// region is large enough to hold every array it declares.
    ///
    /// # Parameters
    ///
    /// - `base`: Pointer to the first byte of the `.gnu.hash` section.
    /// - `byte_len`: Size of the `.gnu.hash` section in bytes.
    /// - `symtab_len`: Number of entries in the associated symbol table (bounds
    ///   the chain array, whose length is `symtab_len - symoffset`).
    ///
    /// # Returns
    ///
    /// A [`GnuHashTable`] on success, or [`None`] if the header is degenerate or
    /// the region is too small for the declared layout.
    ///
    /// # Safety
    ///
    /// `base` must point to `byte_len` readable bytes that remain valid for the
    /// lifetime of the returned table.
    ///
    pub unsafe fn from_raw_parts(
        base: *const u8,
        byte_len: usize,
        symtab_len: usize,
    ) -> Option<Self> {
        // Header is four u32 words.
        if byte_len < 16 {
            return None;
        }
        let nbuckets: u32 = read_u32(base, 0);
        let symoffset: u32 = read_u32(base, 4);
        let bloom_size: u32 = read_u32(base, 8);
        let bloom_shift: u32 = read_u32(base, 12);

        // Guard against degenerate headers that would divide by zero, shift by
        // an out-of-range amount, or place the first hashed symbol past the end
        // of the symbol table.
        if nbuckets == 0
            || bloom_size == 0
            || bloom_shift >= u32::BITS
            || symoffset as usize > symtab_len
        {
            return None;
        }

        // Validate that the region holds header + bloom[] + bucket[] + chain[].
        let chain_words: u64 = (symtab_len as u64).checked_sub(symoffset as u64)?;
        let total: u64 = 16u64
            .checked_add((bloom_size as u64).checked_mul(ELFCLASS_BYTES as u64)?)?
            .checked_add((nbuckets as u64).checked_mul(4)?)?
            .checked_add(chain_words.checked_mul(4)?)?;
        if (byte_len as u64) < total {
            return None;
        }

        Some(GnuHashTable {
            base,
            nbuckets,
            symoffset,
            bloom_size,
            bloom_shift,
        })
    }

    /// Reads Bloom-filter word `i`. The caller must ensure `i < bloom_size`.
    #[inline]
    fn bloom(&self, i: u32) -> ElfBloomWord {
        // SAFETY: validated by `from_raw_parts`; `i < bloom_size` by contract.
        unsafe { read_bloom_word(self.base, 16 + i as usize * ELFCLASS_BYTES) }
    }

    /// Reads `bucket[i]`. The caller must ensure `i < nbuckets`.
    #[inline]
    fn bucket(&self, i: u32) -> u32 {
        let off: usize = 16 + self.bloom_size as usize * ELFCLASS_BYTES + i as usize * 4;
        // SAFETY: validated by `from_raw_parts`; `i < nbuckets` by contract.
        unsafe { read_u32(self.base, off) }
    }

    /// Reads the chain hash for symbol index `sym_idx`. The caller must ensure
    /// `symoffset <= sym_idx < symtab_len`.
    #[inline]
    fn chain(&self, sym_idx: u32) -> u32 {
        let off: usize = 16
            + self.bloom_size as usize * ELFCLASS_BYTES
            + self.nbuckets as usize * 4
            + (sym_idx - self.symoffset) as usize * 4;
        // SAFETY: validated by `from_raw_parts`; the index is in range by the
        // caller's contract.
        unsafe { read_u32(self.base, off) }
    }

    ///
    /// # Description
    ///
    /// Looks up a symbol by name using the GNU hash table.
    ///
    /// The Bloom filter rejects most absent names outright; otherwise the bucket
    /// selects a chain of candidates sharing the top 31 hash bits, and
    /// `name_matches` confirms the exact name.
    ///
    /// # Parameters
    ///
    /// - `name`: The raw symbol-name bytes to look up (without a NUL).
    /// - `symtab_len`: Number of entries in the associated symbol table.
    /// - `name_matches`: Predicate returning `true` when the symbol at the given
    ///   index has the sought name.
    ///
    /// # Returns
    ///
    /// A [`Lookup`] describing whether the symbol was found, is provably absent,
    /// or the table is inconsistent.
    ///
    pub fn lookup<F>(&self, name: &[u8], symtab_len: usize, mut name_matches: F) -> Lookup
    where
        F: FnMut(u32) -> bool,
    {
        let hash: u32 = gnu_hash(name);

        // Bloom-filter probe: if either bit is clear the symbol is definitely
        // absent from this object.
        let word: u32 = (hash / ELFCLASS_BITS) % self.bloom_size;
        let bloom_word: ElfBloomWord = self.bloom(word);
        let mask: ElfBloomWord = ((1 as ElfBloomWord) << (hash % ELFCLASS_BITS))
            | ((1 as ElfBloomWord) << ((hash >> self.bloom_shift) % ELFCLASS_BITS));
        if bloom_word & mask != mask {
            return Lookup::NotFound;
        }

        // Walk the bucket's chain of candidates.
        let mut idx: u32 = self.bucket(hash % self.nbuckets);
        if idx == STN_UNDEF {
            // Empty bucket: no symbol hashes into this slot.
            return Lookup::NotFound;
        }
        if idx < self.symoffset {
            // A nonzero index below the first hashed symbol is corrupt.
            return Lookup::Inconsistent;
        }
        loop {
            if idx as usize >= symtab_len {
                return Lookup::Inconsistent;
            }
            let chain_hash: u32 = self.chain(idx);
            // Chain entries store the hash with its low bit repurposed as the
            // end-of-chain flag, so compare on the top 31 bits.
            if (chain_hash | 1) == (hash | 1) && name_matches(idx) {
                return Lookup::Found(idx);
            }
            if chain_hash & 1 != 0 {
                return Lookup::NotFound;
            }
            idx += 1;
        }
    }
}

//==================================================================================================
// SymbolHashTable
//==================================================================================================

///
/// # Description
///
/// A symbol-lookup accelerator built from whichever ELF hash table an object
/// provides. `DT_GNU_HASH` is preferred when both are present.
///
#[derive(Debug)]
pub enum SymbolHashTable {
    /// A System V (`.hash`) table.
    Sysv(SysvHashTable),
    /// A GNU (`.gnu.hash`) table.
    Gnu(GnuHashTable),
}

impl SymbolHashTable {
    ///
    /// # Description
    ///
    /// Looks up a symbol by name through the underlying hash table.
    ///
    /// # Parameters
    ///
    /// - `name`: The raw symbol-name bytes to look up (without a NUL).
    /// - `symtab_len`: Number of entries in the associated symbol table.
    /// - `name_matches`: Predicate confirming the symbol at a candidate index has
    ///   the sought name.
    ///
    /// # Returns
    ///
    /// A [`Lookup`] describing whether the symbol was found, is provably absent,
    /// or the table is inconsistent.
    ///
    pub fn lookup<F>(&self, name: &[u8], symtab_len: usize, name_matches: F) -> Lookup
    where
        F: FnMut(u32) -> bool,
    {
        match self {
            SymbolHashTable::Sysv(table) => table.lookup(name, symtab_len, name_matches),
            SymbolHashTable::Gnu(table) => table.lookup(name, symtab_len, name_matches),
        }
    }
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(test)]
mod tests;
