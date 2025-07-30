use bincode::{encode_to_vec, decode_from_slice, config::standard, Decode, Encode};

pub fn serialize_payload<T: Encode>(payload: &T) -> Vec<u8> {
    encode_to_vec(payload, standard()).expect("serialization failed")
}

pub fn deserialize_payload<T: Decode<()>>(data: &[u8]) -> T {
    decode_from_slice::<T, _>(data, standard())
        .expect("deserialization failed")
        .0
}


