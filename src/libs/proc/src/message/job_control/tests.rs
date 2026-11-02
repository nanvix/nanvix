// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;

#[test]
fn job_control_op_round_trips_through_byte() {
    for op in [
        JobControlOp::SetSid,
        JobControlOp::SetPgid,
        JobControlOp::GetPgid,
        JobControlOp::GetSid,
        JobControlOp::TcSetPgrp,
        JobControlOp::TcGetPgrp,
    ] {
        let byte: u8 = op.into();
        assert_eq!(JobControlOp::try_from(byte).expect("valid op"), op);
    }
}

#[test]
fn job_control_op_rejects_unknown_byte() {
    assert!(JobControlOp::try_from(0).is_err());
    assert!(JobControlOp::try_from(7).is_err());
}

#[test]
fn job_control_request_preserves_fields() {
    let pid: ProcessIdentifier = ProcessIdentifier::from(42);
    let pgid: ProcessIdentifier = ProcessIdentifier::from(7);
    let request: JobControlRequest = JobControlRequest::new(JobControlOp::SetPgid, pid, pgid);
    let decoded: JobControlRequest = JobControlRequest::from_bytes(request.into_bytes());

    assert_eq!(decoded.op().expect("valid op"), JobControlOp::SetPgid);
    assert_eq!({ decoded.pid }, pid);
    assert_eq!({ decoded.pgid }, pgid);
}

#[test]
fn job_control_response_preserves_fields() {
    let result: ProcessIdentifier = ProcessIdentifier::from(123);
    let response: JobControlResponse = JobControlResponse::new(0, result);
    let decoded: JobControlResponse = JobControlResponse::from_bytes(response.into_bytes());

    assert_eq!({ decoded.error }, 0);
    assert_eq!({ decoded.result }, result);
}
