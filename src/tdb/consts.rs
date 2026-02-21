pub const MAGIC: [u8; 4] = [0x08, 0xd5, 0x19, 0x87];
pub const HEADER_SIZE: usize = 12;
pub const INDEX_ENTRY_SIZE: usize = 4;
pub const PADDING_SIZE: usize = 8;
pub const RECORD_SIZE: usize = 96;

pub const FLARM_ID_OFFSET: usize = 0;
pub const FREQUENCY_OFFSET: usize = 4;
pub const CALL_SIGN_OFFSET: usize = 16;
pub const PILOT_NAME_OFFSET: usize = 32;
pub const AIRFIELD_OFFSET: usize = 48;
pub const PLANE_TYPE_OFFSET: usize = 64;
pub const REGISTRATION_OFFSET: usize = 80;
pub const STRING_FIELD_SIZE: usize = 16;
