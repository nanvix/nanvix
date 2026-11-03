// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;
use ::sys::{
    error::ErrorCode,
    ipc::{
        MessageReceiver,
        MessageSender,
        MessageType,
    },
    pm::ThreadIdentifier,
};

fn sample_message(seed: u8) -> Message {
    let mut payload: [u8; Message::PAYLOAD_SIZE] = [0; Message::PAYLOAD_SIZE];
    for (i, byte) in payload.iter_mut().enumerate() {
        *byte = seed.wrapping_add(i as u8);
    }
    Message::new(MessageSender::NETWORKD, MessageReceiver::KERNEL, MessageType::Ikc, None, payload)
}

fn sample_error_message() -> Message {
    Message::new(
        MessageSender::NETWORKD,
        MessageReceiver::KERNEL,
        MessageType::Ikc,
        Some(ErrorCode::InvalidArgument),
        [0; Message::PAYLOAD_SIZE],
    )
}

fn message_bytes(msg: &Message) -> [u8; MESSAGE_BYTES] {
    msg.clone().to_bytes()
}

#[test]
fn request_message_roundtrip() {
    let req = NetworkRequest {
        op: NetworkOp::Message(sample_message(1)),
    };
    let decoded = NetworkRequest::decode(&req.encode().unwrap()).unwrap();
    match (decoded.op, req.op) {
        (NetworkOp::Message(a), NetworkOp::Message(b)) => {
            assert_eq!(message_bytes(&a), message_bytes(&b));
        },
        _ => panic!("variant mismatch"),
    }
}

#[test]
fn request_push_roundtrip() {
    let data: Vec<u8> = (0..4096u32).map(|v| v as u8).collect();
    let req = NetworkRequest {
        op: NetworkOp::Push {
            msg: sample_message(7),
            data: data.clone(),
        },
    };
    let decoded = NetworkRequest::decode(&req.encode().unwrap()).unwrap();
    match decoded.op {
        NetworkOp::Push {
            msg,
            data: decoded_data,
        } => {
            assert_eq!(message_bytes(&msg), message_bytes(&sample_message(7)));
            assert_eq!(decoded_data, data);
        },
        _ => panic!("variant mismatch"),
    }
}

#[test]
fn request_pull_roundtrip() {
    let req = NetworkRequest {
        op: NetworkOp::Pull(sample_message(9)),
    };
    let decoded = NetworkRequest::decode(&req.encode().unwrap()).unwrap();
    assert!(matches!(decoded.op, NetworkOp::Pull(_)));
}

#[test]
fn response_message_none_roundtrip() {
    let resp = NetworkResponse {
        tid: ThreadIdentifier::from(5),
        result: NetworkResult::Message(None),
    };
    let decoded = NetworkResponse::decode(&resp.encode().unwrap()).unwrap();
    assert_eq!(decoded.tid, ThreadIdentifier::from(5));
    assert!(matches!(decoded.result, NetworkResult::Message(None)));
}

#[test]
fn response_message_some_roundtrip() {
    let messages = vec![sample_message(1), sample_message(2), sample_message(3)];
    let resp = NetworkResponse {
        tid: ThreadIdentifier::from(5),
        result: NetworkResult::Message(Some(messages.clone())),
    };
    let decoded = NetworkResponse::decode(&resp.encode().unwrap()).unwrap();
    assert_eq!(decoded.tid, ThreadIdentifier::from(5));
    match decoded.result {
        NetworkResult::Message(Some(decoded_messages)) => {
            assert_eq!(decoded_messages.len(), messages.len());
            for (a, b) in decoded_messages.iter().zip(messages.iter()) {
                assert_eq!(message_bytes(a), message_bytes(b));
            }
        },
        _ => panic!("variant mismatch"),
    }
}

#[test]
fn response_push_roundtrip() {
    let resp = NetworkResponse {
        tid: ThreadIdentifier::from(42),
        result: NetworkResult::Push(sample_message(6)),
    };
    let decoded = NetworkResponse::decode(&resp.encode().unwrap()).unwrap();
    assert_eq!(decoded.tid, ThreadIdentifier::from(42));
    match decoded.result {
        NetworkResult::Push(msg) => {
            assert_eq!(message_bytes(&msg), message_bytes(&sample_message(6)));
        },
        _ => panic!("variant mismatch"),
    }
}

#[test]
fn response_pull_roundtrip() {
    let data: Vec<u8> = vec![0xAB; 1500];
    let resp = NetworkResponse {
        tid: ThreadIdentifier::from(77),
        result: NetworkResult::Pull {
            msg: sample_message(4),
            data: data.clone(),
        },
    };
    let decoded = NetworkResponse::decode(&resp.encode().unwrap()).unwrap();
    assert_eq!(decoded.tid, ThreadIdentifier::from(77));
    match decoded.result {
        NetworkResult::Pull {
            msg,
            data: decoded_data,
        } => {
            assert_eq!(message_bytes(&msg), message_bytes(&sample_message(4)));
            assert_eq!(decoded_data, data);
        },
        _ => panic!("variant mismatch"),
    }
}

#[test]
fn response_bulk_roundtrip_supports_errors() {
    let push_msg: Message = sample_error_message();
    let expected_push_msg: [u8; MESSAGE_BYTES] = message_bytes(&push_msg);
    let push_resp = NetworkResponse {
        tid: ThreadIdentifier::from(42),
        result: NetworkResult::Push(push_msg),
    };
    let decoded: NetworkResponse = NetworkResponse::decode(&push_resp.encode().unwrap()).unwrap();
    match decoded.result {
        NetworkResult::Push(msg) => assert_eq!(message_bytes(&msg), expected_push_msg),
        _ => panic!("variant mismatch"),
    }

    let pull_msg: Message = sample_error_message();
    let expected_pull_msg: [u8; MESSAGE_BYTES] = message_bytes(&pull_msg);
    let pull_resp = NetworkResponse {
        tid: ThreadIdentifier::from(77),
        result: NetworkResult::Pull {
            msg: pull_msg,
            data: Vec::new(),
        },
    };
    let decoded: NetworkResponse = NetworkResponse::decode(&pull_resp.encode().unwrap()).unwrap();
    match decoded.result {
        NetworkResult::Pull { msg, data } => {
            assert_eq!(message_bytes(&msg), expected_pull_msg);
            assert!(data.is_empty());
        },
        _ => panic!("variant mismatch"),
    }
}

#[test]
fn response_pull_max_payload_reaches_max_frame_size() {
    let resp = NetworkResponse {
        tid: ThreadIdentifier::from(77),
        result: NetworkResult::Pull {
            msg: sample_message(4),
            data: vec![0xAB; MAX_PAYLOAD_BYTES],
        },
    };

    let body: Vec<u8> = resp.encode().unwrap();
    assert_eq!(body.len(), MAX_FRAME_BYTES);
}

#[test]
fn decode_rejects_truncated() {
    assert_eq!(NetworkRequest::decode(&[]).unwrap_err(), WireError::Truncated);
    assert_eq!(
        NetworkRequest::decode(&[WireOp::Message.to_u8()]).unwrap_err(),
        WireError::Truncated
    );
}

#[test]
fn decode_rejects_invalid_op() {
    assert_eq!(NetworkRequest::decode(&[0xFF]).unwrap_err(), WireError::InvalidOp(0xFF));
}

#[test]
fn decode_rejects_invalid_embedded_message() {
    let mut msg: [u8; MESSAGE_BYTES] = sample_message(1).to_bytes();
    msg[0] = 0xFF;

    let mut body: Vec<u8> = Vec::new();
    body.push(WireOp::Message.to_u8());
    body.extend_from_slice(&msg);

    assert_eq!(NetworkRequest::decode(&body).unwrap_err(), WireError::InvalidMessage);
}

#[test]
fn decode_rejects_trailing_bytes() {
    let mut body: Vec<u8> = NetworkRequest {
        op: NetworkOp::Pull(sample_message(1)),
    }
    .encode()
    .unwrap();
    body.push(0);
    assert_eq!(NetworkRequest::decode(&body).unwrap_err(), WireError::TrailingBytes);
}

#[test]
fn request_decode_rejects_oversized_body() {
    let body: Vec<u8> = vec![0; MAX_FRAME_BYTES + 1];
    assert_eq!(
        NetworkRequest::decode(&body).unwrap_err(),
        WireError::FrameTooLarge(MAX_FRAME_BYTES + 1)
    );
}

#[test]
fn response_decode_rejects_oversized_body() {
    let body: Vec<u8> = vec![0; MAX_FRAME_BYTES + 1];
    assert_eq!(
        NetworkResponse::decode(&body).unwrap_err(),
        WireError::FrameTooLarge(MAX_FRAME_BYTES + 1)
    );
}

#[test]
fn encode_rejects_oversized_payload() {
    let req = NetworkRequest {
        op: NetworkOp::Push {
            msg: sample_message(1),
            data: vec![0; MAX_PAYLOAD_BYTES + 1],
        },
    };

    assert_eq!(req.encode().unwrap_err(), WireError::PayloadTooLarge(MAX_PAYLOAD_BYTES + 1));
}

#[test]
fn request_decode_rejects_oversized_payload_before_allocating() {
    let mut body: Vec<u8> = Vec::new();
    body.push(WireOp::Push.to_u8());
    body.extend_from_slice(&sample_message(1).to_bytes());
    body.extend_from_slice(&(u32::try_from(MAX_PAYLOAD_BYTES).unwrap() + 1).to_le_bytes());

    assert_eq!(
        NetworkRequest::decode(&body).unwrap_err(),
        WireError::PayloadTooLarge(MAX_PAYLOAD_BYTES + 1)
    );
}

#[test]
fn response_encode_rejects_oversized_payload() {
    let resp = NetworkResponse {
        tid: ThreadIdentifier::from(5),
        result: NetworkResult::Pull {
            msg: sample_message(1),
            data: vec![0; MAX_PAYLOAD_BYTES + 1],
        },
    };

    assert_eq!(resp.encode().unwrap_err(), WireError::PayloadTooLarge(MAX_PAYLOAD_BYTES + 1));
}

#[test]
fn response_decode_rejects_oversized_payload_before_allocating() {
    let mut body: Vec<u8> = Vec::new();
    body.push(WireOp::Pull.to_u8());
    body.extend_from_slice(&i32::from(ThreadIdentifier::from(5)).to_le_bytes());
    body.extend_from_slice(&sample_message(1).to_bytes());
    body.extend_from_slice(&(u32::try_from(MAX_PAYLOAD_BYTES).unwrap() + 1).to_le_bytes());

    assert_eq!(
        NetworkResponse::decode(&body).unwrap_err(),
        WireError::PayloadTooLarge(MAX_PAYLOAD_BYTES + 1)
    );
}

#[test]
fn message_count_error_reports_byte_size() {
    let count: usize = MAX_FRAME_BYTES / MESSAGE_BYTES + 1;
    let len: usize = count * MESSAGE_BYTES;
    assert_eq!(message_count_to_u32(count).unwrap_err(), WireError::FrameTooLarge(len));
}

#[test]
fn response_decode_rejects_impossible_message_count_before_allocating() {
    let mut body: Vec<u8> = Vec::new();
    body.push(WireOp::Message.to_u8());
    body.extend_from_slice(&i32::from(ThreadIdentifier::from(5)).to_le_bytes());
    body.push(Presence::Some.to_u8());
    body.extend_from_slice(&u32::MAX.to_le_bytes());
    assert_eq!(NetworkResponse::decode(&body).unwrap_err(), WireError::Truncated);
}

#[test]
fn response_decode_rejects_invalid_presence_flag() {
    let mut body: Vec<u8> = Vec::new();
    body.push(WireOp::Message.to_u8());
    body.extend_from_slice(&i32::from(ThreadIdentifier::from(5)).to_le_bytes());
    body.push(2);
    assert_eq!(NetworkResponse::decode(&body).unwrap_err(), WireError::InvalidPresence(2));
}

#[test]
fn response_decode_rejects_invalid_op_in_one_byte_body() {
    assert_eq!(NetworkResponse::decode(&[0xFF]).unwrap_err(), WireError::InvalidOp(0xFF));
}

#[test]
fn request_to_frame_is_length_prefixed() {
    let req = NetworkRequest {
        op: NetworkOp::Message(sample_message(1)),
    };
    let body: Vec<u8> = req.encode().unwrap();
    let frame: Vec<u8> = req.to_frame().unwrap();
    assert_eq!(frame.len(), LENGTH_PREFIX_BYTES + body.len());
    let len: u32 = u32::from_le_bytes([frame[0], frame[1], frame[2], frame[3]]);
    assert_eq!(usize::try_from(len).unwrap(), body.len());
    assert_eq!(&frame[LENGTH_PREFIX_BYTES..], body.as_slice());
    // The framed body must decode back to an equivalent request.
    let decoded = NetworkRequest::decode(&frame[LENGTH_PREFIX_BYTES..]).unwrap();
    assert!(matches!(decoded.op, NetworkOp::Message(_)));
}

#[test]
fn response_to_frame_is_length_prefixed() {
    let resp = NetworkResponse {
        tid: ThreadIdentifier::from(11),
        result: NetworkResult::Message(None),
    };
    let body: Vec<u8> = resp.encode().unwrap();
    let frame: Vec<u8> = resp.to_frame().unwrap();
    assert_eq!(frame.len(), LENGTH_PREFIX_BYTES + body.len());
    let len: u32 = u32::from_le_bytes([frame[0], frame[1], frame[2], frame[3]]);
    assert_eq!(usize::try_from(len).unwrap(), body.len());
    assert_eq!(&frame[LENGTH_PREFIX_BYTES..], body.as_slice());
}
