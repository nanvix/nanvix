pub fn ex_error_new(code: ErrorCode, reason: &'static str) -> (result: Error)
    ensures
        result.code == code,
        result.reason == reason,
{
    Error::new(code, reason)
}
