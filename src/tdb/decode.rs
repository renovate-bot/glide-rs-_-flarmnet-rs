use super::consts::*;
use crate::Record;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum DecodeError {
    #[error("unexpected end of file")]
    UnexpectedEof,
    #[error("invalid magic number: {0:02x?}")]
    InvalidMagic([u8; 4]),
    #[error("invalid FLARM id: {0}")]
    InvalidFlarmId(u32),
    #[error("invalid UTF-8 in {field} field at record offset {offset}")]
    InvalidUtf8 { field: &'static str, offset: usize },
}

#[derive(Debug)]
pub struct DecodedFile {
    pub version: u32,
    pub records: Vec<Result<Record, DecodeError>>,
}

pub fn decode_file(data: &[u8]) -> Result<DecodedFile, DecodeError> {
    if data.len() < HEADER_SIZE {
        return Err(DecodeError::UnexpectedEof);
    }

    let magic: [u8; 4] = data[0..4].try_into().unwrap();
    if magic != MAGIC {
        return Err(DecodeError::InvalidMagic(magic));
    }

    let version = u32::from_le_bytes(data[4..8].try_into().unwrap());
    let record_count = u32::from_le_bytes(data[8..12].try_into().unwrap()) as usize;

    let expected_size =
        HEADER_SIZE + record_count * INDEX_ENTRY_SIZE + PADDING_SIZE + record_count * RECORD_SIZE;
    if data.len() < expected_size {
        return Err(DecodeError::UnexpectedEof);
    }

    let records_offset = HEADER_SIZE + record_count * INDEX_ENTRY_SIZE + PADDING_SIZE;

    let records = (0..record_count)
        .map(|i| {
            let offset = records_offset + i * RECORD_SIZE;
            let record_data: &[u8; 96] = data[offset..offset + RECORD_SIZE].try_into().unwrap();
            decode_record(record_data)
        })
        .collect();

    Ok(DecodedFile { version, records })
}

fn decode_record(data: &[u8; 96]) -> Result<Record, DecodeError> {
    let flarm_id = u32::from_le_bytes(
        data[FLARM_ID_OFFSET..FLARM_ID_OFFSET + 4]
            .try_into()
            .unwrap(),
    );
    if flarm_id > 0xFFFFFF {
        return Err(DecodeError::InvalidFlarmId(flarm_id));
    }
    let flarm_id = format!("{:06X}", flarm_id);

    let frequency = u32::from_le_bytes(
        data[FREQUENCY_OFFSET..FREQUENCY_OFFSET + 4]
            .try_into()
            .unwrap(),
    );
    let frequency = if frequency == 0 {
        String::new()
    } else {
        format!("{}.{:03}", frequency / 1000, frequency % 1000)
    };

    let call_sign = decode_string(data, CALL_SIGN_OFFSET, "call_sign")?;
    let pilot_name = decode_string(data, PILOT_NAME_OFFSET, "pilot_name")?;
    let airfield = decode_string(data, AIRFIELD_OFFSET, "airfield")?;
    let plane_type = decode_string(data, PLANE_TYPE_OFFSET, "plane_type")?;
    let registration = decode_string(data, REGISTRATION_OFFSET, "registration")?;

    Ok(Record {
        flarm_id,
        pilot_name,
        airfield,
        plane_type,
        registration,
        call_sign,
        frequency,
    })
}

fn decode_string(
    data: &[u8; 96],
    offset: usize,
    field: &'static str,
) -> Result<String, DecodeError> {
    let field_bytes = &data[offset..offset + STRING_FIELD_SIZE];

    let end = field_bytes
        .iter()
        .position(|&b| b == 0)
        .unwrap_or(STRING_FIELD_SIZE);
    let content = &field_bytes[..end];

    std::str::from_utf8(content)
        .map(|s| s.to_string())
        .map_err(|_| DecodeError::InvalidUtf8 { field, offset })
}

#[cfg(test)]
mod tests {
    use super::*;
    use insta::assert_debug_snapshot;

    #[test]
    fn decoding_fails_for_empty_file() {
        assert_debug_snapshot!(decode_file(b"").unwrap_err(), @"UnexpectedEof");
    }

    #[test]
    fn decoding_fails_for_truncated_header() {
        assert_debug_snapshot!(
            decode_file(&[0x08, 0xd5, 0x19]).unwrap_err(),
            @"UnexpectedEof"
        );
    }

    #[test]
    fn decoding_fails_for_invalid_magic() {
        let data = [
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ];
        assert_debug_snapshot!(
            decode_file(&data).unwrap_err(),
            @r###"
        InvalidMagic(
            [
                0,
                0,
                0,
                0,
            ],
        )
        "###
        );
    }

    #[test]
    fn decoding_fails_for_truncated_records() {
        // valid header claiming 1 record, but no index/padding/record data
        let mut data = vec![0x08, 0xd5, 0x19, 0x87]; // magic
        data.extend_from_slice(&1u32.to_le_bytes()); // version
        data.extend_from_slice(&1u32.to_le_bytes()); // record count = 1
        assert_debug_snapshot!(decode_file(&data).unwrap_err(), @"UnexpectedEof");
    }

    fn make_valid_file(records: &[[u8; RECORD_SIZE]]) -> Vec<u8> {
        let n = records.len() as u32;
        let mut data = Vec::new();

        // header
        data.extend_from_slice(&MAGIC);
        data.extend_from_slice(&1u32.to_le_bytes()); // version = 1
        data.extend_from_slice(&n.to_le_bytes());

        // index (dummy sorted flarm_ids)
        for record in records {
            let flarm_id = u32::from_le_bytes(record[0..4].try_into().unwrap());
            data.extend_from_slice(&flarm_id.to_le_bytes());
        }

        // padding
        data.extend_from_slice(&[0u8; PADDING_SIZE]);

        // records
        for record in records {
            data.extend_from_slice(record);
        }

        data
    }

    fn make_record(
        flarm_id: u32,
        frequency: u32,
        call_sign: &[u8],
        airfield: &[u8],
        plane_type: &[u8],
        registration: &[u8],
    ) -> [u8; RECORD_SIZE] {
        let mut record = [0u8; RECORD_SIZE];
        record[FLARM_ID_OFFSET..FLARM_ID_OFFSET + 4].copy_from_slice(&flarm_id.to_le_bytes());
        record[FREQUENCY_OFFSET..FREQUENCY_OFFSET + 4].copy_from_slice(&frequency.to_le_bytes());

        let fields: [(&[u8], usize); 4] = [
            (call_sign, CALL_SIGN_OFFSET),
            (airfield, AIRFIELD_OFFSET),
            (plane_type, PLANE_TYPE_OFFSET),
            (registration, REGISTRATION_OFFSET),
        ];
        for (value, offset) in &fields {
            let len = value.len().min(STRING_FIELD_SIZE - 1);
            record[*offset..*offset + len].copy_from_slice(&value[..len]);
        }

        record
    }

    #[test]
    fn decoding_works_for_empty_database() {
        let data = make_valid_file(&[]);
        let result = decode_file(&data).unwrap();
        assert_eq!(result.version, 1);
        assert_eq!(result.records.len(), 0);
    }

    #[test]
    fn decoding_works_for_single_record() {
        let record = make_record(0x3EE3C7, 123500, b"SG", b"EDKA", b"LS6a", b"D-0816");
        let data = make_valid_file(&[record]);
        let result = decode_file(&data).unwrap();
        assert_eq!(result.version, 1);
        assert_eq!(result.records.len(), 1);
        assert_debug_snapshot!(result.records[0].as_ref().unwrap(), @r###"
        Record {
            flarm_id: "3EE3C7",
            pilot_name: "",
            airfield: "EDKA",
            plane_type: "LS6a",
            registration: "D-0816",
            call_sign: "SG",
            frequency: "123.500",
        }
        "###);
    }

    #[test]
    fn decoding_works_with_zero_frequency() {
        let record = make_record(0x000001, 0, b"", b"", b"Paraglider", b"");
        let data = make_valid_file(&[record]);
        let result = decode_file(&data).unwrap();
        let record = result.records[0].as_ref().unwrap();
        assert_eq!(record.frequency, "");
    }

    #[test]
    fn decoding_reports_invalid_flarm_id() {
        let mut record = [0u8; RECORD_SIZE];
        record[0..4].copy_from_slice(&0x01000000u32.to_le_bytes()); // > 0xFFFFFF
        let data = make_valid_file(&[record]);
        let result = decode_file(&data).unwrap();
        assert_debug_snapshot!(
            result.records[0].as_ref().unwrap_err(),
            @r###"
        InvalidFlarmId(
            16777216,
        )
        "###
        );
    }

    #[test]
    fn decoding_reports_invalid_utf8() {
        let mut record = make_record(0x000001, 0, b"", b"", b"", b"");
        // put invalid UTF-8 in the call_sign field
        record[CALL_SIGN_OFFSET] = 0xFF;
        record[CALL_SIGN_OFFSET + 1] = 0xFE;
        let data = make_valid_file(&[record]);
        let result = decode_file(&data).unwrap();
        assert_debug_snapshot!(
            result.records[0].as_ref().unwrap_err(),
            @r###"
        InvalidUtf8 {
            field: "call_sign",
            offset: 16,
        }
        "###
        );
    }
}
