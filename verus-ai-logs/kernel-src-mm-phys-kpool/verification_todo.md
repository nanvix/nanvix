# Verification TODOs: kpool

No proof gaps remain. All four `Inner` methods (`new`, `alloc`, `alloc_range`, `free`)
are fully body-verified with zero `admit`, `assume`, `external_body`, or
`assume_specification` in the kpool source, spec, and proof files.
