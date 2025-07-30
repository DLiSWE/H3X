#[repr(u8)]
#[derive(Debug, Clone, Copy)]
pub enum FrameType {
    Ping = 1,
    Pong = 2,
    Auth = 3,
    Event = 4,
}

impl From<u8> for FrameType {
    fn from(byte: u8) -> Self {
        match byte {
            1 => FrameType::Ping,
            2 => FrameType::Pong,
            3 => FrameType::Auth,
            4 => FrameType::Event,
            _ => FrameType::Ping,
        }
    }
}

