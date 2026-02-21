use super::consts::*;
use crate::{File, Record};
use std::io::{Cursor, Write};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum EncodeError {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error("invalid FLARM id: {0}")]
    InvalidFlarmId(String),
    #[error("invalid frequency: {0}")]
    InvalidFrequency(String),
}

pub fn encode_file(file: &File) -> Result<Vec<u8>, EncodeError> {
    let mut writer = Writer::new(Cursor::new(Vec::new()));
    writer.write(file)?;
    Ok(writer.into_inner().into_inner())
}

pub struct Writer<W: Write> {
    writer: W,
}

impl<W: Write> Writer<W> {
    pub fn new(inner: W) -> Self {
        Self { writer: inner }
    }

    pub fn write(&mut self, file: &File) -> Result<(), EncodeError> {
        let mut entries: Vec<(u32, &Record)> = file
            .records
            .iter()
            .map(|record| Ok((parse_flarm_id(&record.flarm_id)?, record)))
            .collect::<Result<_, EncodeError>>()?;

        entries.sort_by_key(|(id, _)| *id);

        let count = entries.len() as u32;

        // header
        self.writer.write_all(&MAGIC)?;
        self.writer.write_all(&file.version.to_le_bytes())?;
        self.writer.write_all(&count.to_le_bytes())?;

        // index
        for (id, _) in &entries {
            self.writer.write_all(&id.to_le_bytes())?;
        }

        // padding
        self.writer.write_all(&[0u8; PADDING_SIZE])?;

        // records
        for (id, record) in &entries {
            self.write_record(*id, record)?;
        }

        Ok(())
    }

    fn write_record(&mut self, flarm_id: u32, record: &Record) -> Result<(), EncodeError> {
        let frequency = parse_frequency(&record.frequency)?;

        let mut buf = [0u8; RECORD_SIZE];
        buf[FLARM_ID_OFFSET..FLARM_ID_OFFSET + 4].copy_from_slice(&flarm_id.to_le_bytes());
        buf[FREQUENCY_OFFSET..FREQUENCY_OFFSET + 4].copy_from_slice(&frequency.to_le_bytes());
        // reserved at offset 8..16 stays zero
        write_string(&mut buf, CALL_SIGN_OFFSET, &record.call_sign);
        write_string(&mut buf, PILOT_NAME_OFFSET, &record.pilot_name);
        write_string(&mut buf, AIRFIELD_OFFSET, &record.airfield);
        write_string(&mut buf, PLANE_TYPE_OFFSET, &record.plane_type);
        write_string(&mut buf, REGISTRATION_OFFSET, &record.registration);

        self.writer.write_all(&buf)?;
        Ok(())
    }

    pub fn into_inner(self) -> W {
        self.writer
    }
}

fn write_string(buf: &mut [u8; RECORD_SIZE], offset: usize, value: &str) {
    let max_content = STRING_FIELD_SIZE - 1;
    let truncated = if value.len() > max_content {
        &value[..value.floor_char_boundary(max_content)]
    } else {
        value
    };
    buf[offset..offset + truncated.len()].copy_from_slice(truncated.as_bytes());
    // remaining bytes are already zero from initialization
}

fn parse_flarm_id(s: &str) -> Result<u32, EncodeError> {
    let id = u32::from_str_radix(s, 16).map_err(|_| EncodeError::InvalidFlarmId(s.to_string()))?;
    if id > 0xFFFFFF {
        return Err(EncodeError::InvalidFlarmId(s.to_string()));
    }
    Ok(id)
}

fn parse_frequency(s: &str) -> Result<u32, EncodeError> {
    if s.is_empty() {
        return Ok(0);
    }
    let mhz: f64 = s
        .parse()
        .map_err(|_| EncodeError::InvalidFrequency(s.to_string()))?;
    Ok((mhz * 1000.0).round() as u32)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tdb::decode_file;
    use insta::assert_debug_snapshot;

    fn make_file(records: Vec<Record>) -> File {
        File {
            version: 1,
            records,
        }
    }

    fn make_record(
        flarm_id: &str,
        frequency: &str,
        call_sign: &str,
        pilot_name: &str,
        airfield: &str,
        plane_type: &str,
        registration: &str,
    ) -> Record {
        Record {
            flarm_id: flarm_id.to_string(),
            frequency: frequency.to_string(),
            call_sign: call_sign.to_string(),
            pilot_name: pilot_name.to_string(),
            airfield: airfield.to_string(),
            plane_type: plane_type.to_string(),
            registration: registration.to_string(),
        }
    }

    #[test]
    fn encoding_round_trips() {
        let file = make_file(vec![make_record(
            "3EE3C7", "123.500", "SG", "John Doe", "EDKA", "LS6a", "D-0816",
        )]);
        let encoded = encode_file(&file).unwrap();
        let decoded = decode_file(&encoded).unwrap();
        assert_eq!(decoded.version, 1);
        assert_eq!(decoded.records.len(), 1);
        let record = decoded.records[0].as_ref().unwrap();
        assert_eq!(record.flarm_id, "3EE3C7");
        assert_eq!(record.frequency, "123.500");
        assert_eq!(record.call_sign, "SG");
        assert_eq!(record.pilot_name, "John Doe");
        assert_eq!(record.airfield, "EDKA");
        assert_eq!(record.plane_type, "LS6a");
        assert_eq!(record.registration, "D-0816");
    }

    #[test]
    fn encoding_handles_empty_frequency() {
        let file = make_file(vec![make_record(
            "000001",
            "",
            "",
            "",
            "",
            "Paraglider",
            "",
        )]);
        let encoded = encode_file(&file).unwrap();
        let decoded = decode_file(&encoded).unwrap();
        let record = decoded.records[0].as_ref().unwrap();
        assert_eq!(record.frequency, "");
    }

    #[test]
    fn encoding_sorts_records_by_flarm_id() {
        let file = make_file(vec![
            make_record("00000F", "", "X27", "", "D-9527", "ASW 27", "D-9527"),
            make_record("000001", "", "", "", "", "Paraglider", ""),
            make_record("000000", "123.150", "", "", "D-2188", "ASK-13", "D-2188"),
        ]);
        let encoded = encode_file(&file).unwrap();
        let decoded = decode_file(&encoded).unwrap();
        let ids: Vec<&str> = decoded
            .records
            .iter()
            .map(|r| r.as_ref().unwrap().flarm_id.as_str())
            .collect();
        assert_eq!(ids, vec!["000000", "000001", "00000F"]);
    }

    #[test]
    fn encoding_truncates_long_strings() {
        let file = make_file(vec![make_record(
            "000001",
            "",
            "0123456789ABCDEF",
            "",
            "",
            "",
            "",
        )]);
        let encoded = encode_file(&file).unwrap();
        let decoded = decode_file(&encoded).unwrap();
        let record = decoded.records[0].as_ref().unwrap();
        assert_eq!(record.call_sign, "0123456789ABCDE");
    }

    #[test]
    fn encoding_truncates_at_char_boundary() {
        // "Ä" is 2 bytes in UTF-8, so 14 ASCII + "Ä" = 16 bytes, exceeds 15
        let file = make_file(vec![make_record(
            "000001",
            "",
            "01234567890123Ä",
            "",
            "",
            "",
            "",
        )]);
        let encoded = encode_file(&file).unwrap();
        let decoded = decode_file(&encoded).unwrap();
        let record = decoded.records[0].as_ref().unwrap();
        assert_eq!(record.call_sign, "01234567890123");
    }

    #[test]
    fn encoding_fails_for_invalid_flarm_id() {
        let file = make_file(vec![make_record("ZZZZZZ", "", "", "", "", "", "")]);
        assert_debug_snapshot!(
            encode_file(&file).unwrap_err(),
            @r###"
        InvalidFlarmId(
            "ZZZZZZ",
        )
        "###
        );
    }

    #[test]
    fn encoding_fails_for_flarm_id_too_large() {
        let file = make_file(vec![make_record("1000000", "", "", "", "", "", "")]);
        assert_debug_snapshot!(
            encode_file(&file).unwrap_err(),
            @r###"
        InvalidFlarmId(
            "1000000",
        )
        "###
        );
    }

    #[test]
    fn encoding_fails_for_invalid_frequency() {
        let file = make_file(vec![make_record("000001", "abc", "", "", "", "", "")]);
        assert_debug_snapshot!(
            encode_file(&file).unwrap_err(),
            @r###"
        InvalidFrequency(
            "abc",
        )
        "###
        );
    }
}
