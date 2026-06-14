# Verification TODOs: arch-x86-pde

None. Zero proof gaps remain.

All in-scope cheating (2 cfg-gated exec items in `pde.rs`) has been eliminated.
`make verify-arch` reports 47 verified, 0 errors with admit=0, assume=0, cfg_gate=0.

The 3 remaining crate-wide `external_body` items (`invlpg`, `Table::read`,
`Table::write`) are out of scope and are on the TCB-allowed list
(`verus-ai-logs/tcb-allowed.md`).
