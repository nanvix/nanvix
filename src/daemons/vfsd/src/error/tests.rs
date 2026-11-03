// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;

#[test]
fn response_context_stamps_request_id_and_exact_thread() {
    let process: ProcessIdentifier = ProcessIdentifier::from(10);
    let thread: ThreadIdentifier = ThreadIdentifier::from(20);
    let request_id: RequestIdentifier = RequestIdentifier::from_raw(0x12345678);
    let response_context: ResponseContext =
        ResponseContext::new(MessageSender::new(process, thread), request_id);
    let mut response: Message = build_error(thread, ErrorCode::InvalidMessage);

    response_context.prepare_response(&mut response);

    assert_eq!(
        { response.destination },
        MessageReceiver::new(process, thread),
        "response should target the requesting thread"
    );
    assert_eq!(
        RequestIdentifier::read_from(&response),
        request_id,
        "response should echo the request identifier"
    );
}
