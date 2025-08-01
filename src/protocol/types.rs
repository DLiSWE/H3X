#[repr(u8)]
#[derive(Debug, Clone, Copy)]
pub enum FrameType {
    Ping = 1,
    Pong = 2,
    Auth = 3,
    Event = 4,
    FetchEvent = 5,
    EventsBatch = 6,
    AckEvent = 7,
}

impl From<u8> for FrameType {
    fn from(byte: u8) -> Self {
        match byte {
            1 => FrameType::Ping,
            2 => FrameType::Pong,
            3 => FrameType::Auth,
            4 => FrameType::Event,
            5 => FrameType::FetchEvent,
            6 => FrameType::EventsBatch,
            7 => FrameType::AckEvent,
            _ => FrameType::Ping,
        }
    }
}

