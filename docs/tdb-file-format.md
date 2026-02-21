# TDB File Format

Binary format used by Air Avionics devices (AT-1, ATD-57, ATD-80, ATD-11) for
FlarmNet database lookups. Available for download at
<https://www.flarmnet.org/files/downloads/> as `flarmnet.tdb`.

This format was reverse-engineered by cross-referencing a `flarmnet.tdb` file
(14867 records) against the equivalent `data.fln` (XCSoar format) downloaded at
the same time. All flarm IDs, frequencies, and string fields were verified to
match (accounting for field width truncation in the XCSoar format).

## Byte Order

All integers are unsigned, little-endian.

## File Layout

```
Offset          Size            Description
──────          ────            ──────────────────────────────────
0               4 bytes         Magic number: 0x08 0xd5 0x19 0x87
4               4 bytes         Version (u32)
8               4 bytes         Record count N (u32)
12              N × 4 bytes     Flarm ID index
12 + N×4        8 bytes         Padding (zero bytes)
20 + N×4        N × 96 bytes    Record data
```

### Magic Number

The first 4 bytes (`0x08d51987`) are a static format identifier. They do not
change when the file content changes (verified by modifying a record and
re-downloading).

### Version

A `u32` that increments on each file regeneration.

### Flarm ID Index

A sorted array of `u32` flarm IDs. Enables binary search to find a record's
position without scanning the record data. Index entry `i` corresponds to
record `i` in the data section.

### Padding

8 zero bytes separate the index from the record data.

## Record Layout (96 bytes)

```
Offset  Size    Type                Field
──────  ──────  ──────────────────  ──────────────────
0       4       u32                 flarm_id
4       4       u32                 frequency
8       8       reserved            (always zero)
16      16      null-terminated     call_sign
32      16      null-terminated     pilot_name        ⚠ see note below
48      16      null-terminated     airfield
64      16      null-terminated     plane_type
80      16      null-terminated     registration
```

### ⚠ Uncertainty: `pilot_name` Field Offset

The `pilot_name` field is currently mapped to offset 32. However, it is unclear
whether this is correct. The field might actually be at offset 8 (overlapping
what we currently treat as reserved + call_sign). In all observed FlarmNet files
the pilot_name field is empty (zeroed) for privacy reasons, which makes it
impossible to determine the correct offset from the data alone. The current
assignment at offset 32 was chosen based on the uniform 16-byte field size used
by all other string fields, but **this still needs to be verified**, ideally
with a file that contains actual pilot name data.

### Field Details

**flarm_id** — 24-bit FLARM radio ID stored in the low 3 bytes of a `u32`
(max value `0xFFFFFF`). Matches the corresponding entry in the index section.

**frequency** — Radio frequency in kHz as a `u32`. Divide by 1000 to get MHz.
Example: `123500` → `123.500 MHz`. Zero means no frequency set.

**reserved** — 8 bytes, always zero in all observed files. Purpose unknown.

**call_sign** — Up to 15 characters + null terminator, zero-padded. Competition
sign or identifier. Note: the XCSoar `.fln` format truncates this to 3
characters, so the TDB format preserves longer call signs that XCSoar discards.

**pilot_name** — Up to 15 characters + null terminator, zero-padded. Always
empty (zeroed) in FlarmNet-distributed files for privacy reasons.

**airfield** — Up to 15 characters + null terminator, zero-padded. In practice,
FlarmNet stores the registration here instead of an actual airfield name, again
for privacy reasons.

**plane_type** — Up to 15 characters + null terminator, zero-padded. Aircraft
type designation (e.g. "ASK 16", "Discus 2C FES", "HPH 304C Wasp").

**registration** — Up to 15 characters + null terminator, zero-padded. Nearly
always identical to the airfield field in FlarmNet files (14865 of 14867
records matched; the 2 mismatches had U+2013 EN DASH in airfield replaced with
`?` in registration, suggesting a lossy encoding conversion).

### String Encoding

Strings are UTF-8, null-terminated and zero-padded to fill the 16-byte field.
Multi-byte UTF-8 characters have been observed. When encoding, strings longer
than 15 bytes must be truncated at a valid UTF-8 character boundary to avoid
producing invalid output.

## Field Width Comparison

| Field        | TDB           | XCSoar `.fln`      | LX `.fln`     |
|--------------|---------------|--------------------|---------------|
| flarm_id     | u32 (4 bytes) | 6 ASCII hex chars  | XML attribute |
| call_sign    | 16 bytes      | 3 bytes            | XML attribute |
| pilot_name   | 16 bytes      | 21 bytes           | XML attribute |
| airfield     | 16 bytes      | 21 bytes (hex-enc) | XML attribute |
| plane_type   | 16 bytes      | 21 bytes (hex-enc) | XML attribute |
| registration | 16 bytes      | 7 bytes            | XML attribute |
| frequency    | u32 kHz       | 7 ASCII chars      | XML attribute |

The TDB format has wider call_sign (16 vs 3) and registration (16 vs 7) fields
compared to XCSoar, preserving data that the XCSoar format truncates.
