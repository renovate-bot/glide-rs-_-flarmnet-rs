use clap::Parser;
use std::path::PathBuf;

#[derive(Debug, Parser)]
struct Options {
    /// Path to the TDB file
    input: PathBuf,
}

fn main() -> anyhow::Result<()> {
    let options = Options::parse();

    let data = std::fs::read(&options.input)?;
    let decoded = flarmnet::tdb::decode_file(&data)?;

    println!("Version: {}", decoded.version);
    println!("Records: {}", decoded.records.len());

    let ok_count = decoded.records.iter().filter(|r| r.is_ok()).count();
    let err_count = decoded.records.iter().filter(|r| r.is_err()).count();
    println!("  OK: {}", ok_count);
    println!("  Errors: {}", err_count);

    println!();
    println!("First 5 records:");
    for (i, result) in decoded.records.iter().take(5).enumerate() {
        match result {
            Ok(record) => {
                println!(
                    "  [{}] {} call_sign={:?} airfield={:?} plane_type={:?} reg={:?} freq={:?}",
                    i,
                    record.flarm_id,
                    record.call_sign,
                    record.airfield,
                    record.plane_type,
                    record.registration,
                    record.frequency
                );
            }
            Err(e) => {
                println!("  [{}] ERROR: {}", i, e);
            }
        }
    }

    Ok(())
}
