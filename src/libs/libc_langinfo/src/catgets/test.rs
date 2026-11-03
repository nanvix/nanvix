// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::{
    catclose,
    catgets,
    catopen,
    nl_catd,
};

#[test]
fn test_catopen_reports_failure() {
    let catd: nl_catd = catopen(c"messages".as_ptr(), 0);
    assert_eq!(catd, -1isize as nl_catd);
}

#[test]
fn test_catgets_returns_fallback() {
    let fallback = c"fallback".as_ptr();
    let catd: nl_catd = catopen(c"messages".as_ptr(), 0);
    assert_eq!(catgets(catd, 1, 1, fallback).cast_const(), fallback);
}

#[test]
fn test_catclose_succeeds() {
    let catd: nl_catd = catopen(c"messages".as_ptr(), 0);
    assert_eq!(catclose(catd), 0);
}
