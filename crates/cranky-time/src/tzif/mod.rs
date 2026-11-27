//! Reads a zone file in the Time Zone Information Format (RFC 9636).
//!
//! A version 1 file is a header and a block of data with 32-bit instants. Every
//! later version repeats both with 64-bit instants and ends with a POSIX rule
//! between two newlines; only that second copy is read.

use crate::posix_parse;
use crate::zone::{Kind, Leap, Table, Transition, Zone};

const MAGIC: &[u8] = b"TZif";
const RESERVED: usize = 15;
/// The bytes of one kind of local time: offset, daylight flag, name index.
const KIND_SIZE: usize = 6;
/// A transition names its kind in one byte, so a file has no use for more.
const MAX_KINDS: usize = 256;
/// A zone is never further than this from UTC, in seconds.
const OFFSET_RANGE: std::ops::RangeInclusive<i64> = -93_599..=93_599;

/// `None` when the bytes are not one whole, valid zone file.
pub(crate) fn parse(bytes: &[u8]) -> Option<Zone> {
    let mut reader = Reader { rest: bytes };
    let (version, counts) = header(&mut reader)?;
    if version == 0 {
        return Zone::new(block(&mut reader, &counts, 4)?, None);
    }
    reader.take(counts.block_size(4)?)?;
    let (_, counts) = header(&mut reader)?;
    let table = block(&mut reader, &counts, 8)?;
    // A rule this crate cannot read is left out, not refused: the zone is then
    // right up to its last transition, which is what the table itself says.
    let rule = posix_parse::parse(footer(&mut reader)?);
    Zone::new(table, rule)
}

struct Reader<'a> {
    rest: &'a [u8],
}

impl<'a> Reader<'a> {
    fn take(&mut self, count: usize) -> Option<&'a [u8]> {
        let (head, tail) = self.rest.split_at_checked(count)?;
        self.rest = tail;
        Some(head)
    }

    fn count(&mut self) -> Option<usize> {
        let bytes = <[u8; 4]>::try_from(self.take(4)?).ok()?;
        usize::try_from(u32::from_be_bytes(bytes)).ok()
    }
}

/// How many records of each sort a block of data holds.
struct Counts {
    ut_flags: usize,
    standard_flags: usize,
    leaps: usize,
    transitions: usize,
    kinds: usize,
    name_bytes: usize,
}

impl Counts {
    /// The bytes of a block whose instants are `width` bytes wide.
    fn block_size(&self, width: usize) -> Option<usize> {
        self.transitions
            .checked_mul(width.checked_add(1)?)?
            .checked_add(self.kinds.checked_mul(KIND_SIZE)?)?
            .checked_add(self.name_bytes)?
            .checked_add(self.leaps.checked_mul(width.checked_add(4)?)?)?
            .checked_add(self.standard_flags)?
            .checked_add(self.ut_flags)
    }
}

/// The version byte of a header (0 for version 1) and its counts.
fn header(reader: &mut Reader<'_>) -> Option<(u8, Counts)> {
    if reader.take(MAGIC.len())? != MAGIC {
        return None;
    }
    let version = reader.take(1)?.first().copied()?;
    reader.take(RESERVED)?;
    let counts = Counts {
        ut_flags: reader.count()?,
        standard_flags: reader.count()?,
        leaps: reader.count()?,
        transitions: reader.count()?,
        kinds: reader.count()?,
        name_bytes: reader.count()?,
    };
    Some((version, counts))
}

/// A big-endian signed integer of four or eight bytes.
fn integer(bytes: &[u8]) -> Option<i64> {
    <[u8; 4]>::try_from(bytes).map_or_else(
        |_| <[u8; 8]>::try_from(bytes).ok().map(i64::from_be_bytes),
        |narrow| Some(i64::from(i32::from_be_bytes(narrow))),
    )
}

fn block(reader: &mut Reader<'_>, counts: &Counts, width: usize) -> Option<Table> {
    if counts.kinds > MAX_KINDS {
        return None;
    }
    let sections = Sections::take(reader, counts, width)?;
    Some(Table {
        kinds: read_kinds(sections.kinds, sections.names)?,
        transitions: read_transitions(sections.instants, sections.indexes, width)?,
        leaps: read_leaps(sections.leaps, width)?,
    })
}

/// The bytes of each sort of record in a block, in the order the block holds
/// them; the flags that end it are skipped.
struct Sections<'a> {
    instants: &'a [u8],
    indexes: &'a [u8],
    kinds: &'a [u8],
    names: &'a [u8],
    leaps: &'a [u8],
}

impl<'a> Sections<'a> {
    /// Takes a whole block whose instants are `width` bytes wide.
    fn take(reader: &mut Reader<'a>, counts: &Counts, width: usize) -> Option<Self> {
        let instants = reader.take(counts.transitions.checked_mul(width)?)?;
        let indexes = reader.take(counts.transitions)?;
        let kinds = reader.take(counts.kinds.checked_mul(KIND_SIZE)?)?;
        let names = reader.take(counts.name_bytes)?;
        let leaps = reader.take(counts.leaps.checked_mul(leap_size(width)?)?)?;
        reader.take(counts.standard_flags)?;
        reader.take(counts.ut_flags)?;
        Some(Self {
            instants,
            indexes,
            kinds,
            names,
            leaps,
        })
    }
}

/// The kinds of local time in `raw`, their names read from `names`.
fn read_kinds(raw: &[u8], names: &[u8]) -> Option<Vec<Kind>> {
    raw.as_chunks::<KIND_SIZE>()
        .0
        .iter()
        .map(|&raw| kind(raw, names))
        .collect()
}

/// The transitions whose instants, `width` bytes each, are in `instants` and
/// whose kinds are in `indexes`, in the same order.
fn read_transitions(instants: &[u8], indexes: &[u8], width: usize) -> Option<Vec<Transition>> {
    instants
        .chunks_exact(width)
        .zip(indexes)
        .map(|(raw, &index)| {
            Some(Transition {
                instant: integer(raw)?,
                kind: usize::from(index),
            })
        })
        .collect()
}

/// The leap second records in `raw`, each an instant of `width` bytes and then
/// a total of four.
fn read_leaps(raw: &[u8], width: usize) -> Option<Vec<Leap>> {
    raw.chunks_exact(leap_size(width)?)
        .map(|raw| {
            let (instant, total) = raw.split_at_checked(width)?;
            Some(Leap {
                instant: integer(instant)?,
                total: integer(total)?,
            })
        })
        .collect()
}

/// The bytes of one leap second record whose instant is `width` bytes wide: the
/// instant and then a total of four.
const fn leap_size(width: usize) -> Option<usize> {
    width.checked_add(4)
}

fn kind(raw: [u8; KIND_SIZE], names: &[u8]) -> Option<Kind> {
    let [first, second, third, fourth, _is_daylight, name_index] = raw;
    let offset = i64::from(i32::from_be_bytes([first, second, third, fourth]));
    if !OFFSET_RANGE.contains(&offset) {
        return None;
    }
    let name = names.get(usize::from(name_index)..)?;
    let end = name.iter().position(|&byte| byte == 0)?;
    Kind::new(offset, name.get(..end)?)
}

/// The text of the rule after the data, which may be empty. `None` when the two
/// newlines around it are missing, as in a file that is still being written.
fn footer<'a>(reader: &mut Reader<'a>) -> Option<&'a [u8]> {
    if reader.take(1)? != b"\n" {
        return None;
    }
    let end = reader.rest.iter().position(|&byte| byte == b'\n')?;
    reader.take(end)
}

#[cfg(test)]
mod tests;
