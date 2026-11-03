// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;

mod expand_path {
    use super::*;

    /// Tests that bare "~" expands to the home directory.
    #[test]
    fn expand_bare_tilde() {
        let result = expand_path_with_home("~", "/home/user");
        assert_eq!(result, "/home/user");
    }

    /// Tests that "~/subdir" expands to "$HOME/subdir".
    #[test]
    fn expand_tilde_subpath() {
        let result = expand_path_with_home("~/docs/file.txt", "/home/user");
        assert_eq!(result, "/home/user/docs/file.txt");
    }

    /// Tests that a trailing slash in home is not duplicated.
    #[test]
    fn expand_tilde_home_trailing_slash() {
        let result = expand_path_with_home("~/file.txt", "/home/user/");
        assert_eq!(result, "/home/user/file.txt");
    }

    /// Tests that bare "~" with root home returns "/".
    #[test]
    fn expand_bare_tilde_root_home() {
        let result = expand_path_with_home("~", "/");
        assert_eq!(result, "/");
    }

    /// Tests that "~/foo" with root home returns "/foo" (no double slash).
    #[test]
    fn expand_tilde_subpath_root_home() {
        let result = expand_path_with_home("~/foo", "/");
        assert_eq!(result, "/foo");
    }

    /// Tests that absolute paths are returned unchanged.
    #[test]
    fn expand_absolute_path_unchanged() {
        let result = expand_path_with_home("/data/file.txt", "/home/user");
        assert_eq!(result, "/data/file.txt");
    }

    /// Tests that relative paths without tilde are returned unchanged.
    #[test]
    fn expand_relative_path_unchanged() {
        let result = expand_path_with_home("data/file.txt", "/home/user");
        assert_eq!(result, "data/file.txt");
    }

    /// Tests that "~user" form is not expanded (Nanvix is single-user).
    #[test]
    fn expand_tilde_user_not_expanded() {
        let result = expand_path_with_home("~other", "/home/user");
        assert_eq!(result, "~other");
    }

    /// Tests that "~user/path" form is not expanded.
    #[test]
    fn expand_tilde_user_subpath_not_expanded() {
        let result = expand_path_with_home("~other/docs", "/home/user");
        assert_eq!(result, "~other/docs");
    }
}
