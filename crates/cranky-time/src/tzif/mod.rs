//! Reads a zone file in the Time Zone Information Format (RFC 9636).
//!
//! A version 1 file is a header and a block of data with 32-bit instants. Every
//! later version repeats both with 64-bit instants and ends with a POSIX rule
//! between two newlines; only that second copy is read.

use crate::posix_parse;
use crate::zone::{Kind, Leap, Transition, Zone};

const MAGIC: &[u8] = b"TZif";
const RESERVED: usize = 15;
/// The bytes of one kind of local time: offset, daylight flag, name index.
const KIND_SIZE: usize = 6;
/// A zone is never further than this from UTC, in seconds.
const OFFSET_RANGE: std::ops::RangeInclusive<i64> = -93_599..=93_599;

/// `None` when the bytes are not one whole, valid zone file.
pub fn parse(bytes: &[u8]) -> Option<Zone> {
    let mut reader = Reader { rest: bytes };
    let (version, counts) = header(&mut reader)?;
    if version == 0 {
        let (kinds, transitions, leaps) = block(&mut reader, &counts, 4)?;
        return Zone::new(kinds, transitions, leaps, None);
    }
    reader.take(counts.block_size(4)?)?;
    let (_, counts) = header(&mut reader)?;
    let (kinds, transitions, leaps) = block(&mut reader, &counts, 8)?;
    // A rule this crate cannot read is left out, not refused: the zone is then
    // right up to its last transition, which is what the table itself says.
    let rule = posix_parse::parse(footer(&mut reader)?);
    Zone::new(kinds, transitions, leaps, rule)
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

type Block = (Vec<Kind>, Vec<Transition>, Vec<Leap>);

fn block(reader: &mut Reader<'_>, counts: &Counts, width: usize) -> Option<Block> {
    let instants = reader.take(counts.transitions.checked_mul(width)?)?;
    let indexes = reader.take(counts.transitions)?;
    let kinds = reader.take(counts.kinds.checked_mul(KIND_SIZE)?)?;
    let names = reader.take(counts.name_bytes)?;
    let leap_size = width.checked_add(4)?;
    let leaps = reader.take(counts.leaps.checked_mul(leap_size)?)?;
    reader.take(counts.standard_flags)?;
    reader.take(counts.ut_flags)?;

    let kinds = kinds
        .as_chunks::<KIND_SIZE>()
        .0
        .iter()
        .map(|&raw| kind(raw, names))
        .collect::<Option<Vec<_>>>()?;
    let transitions = instants
        .chunks_exact(width)
        .zip(indexes)
        .map(|(at, &index)| {
            Some(Transition {
                at: integer(at)?,
                kind: usize::from(index),
            })
        })
        .collect::<Option<Vec<_>>>()?;
    let leaps = leaps
        .chunks_exact(leap_size)
        .map(|raw| {
            let (at, total) = raw.split_at_checked(width)?;
            Some(Leap {
                at: integer(at)?,
                total: integer(total)?,
            })
        })
        .collect::<Option<Vec<_>>>()?;
    Some((kinds, transitions, leaps))
}

fn kind(raw: [u8; KIND_SIZE], names: &[u8]) -> Option<Kind> {
    let [first, second, third, fourth, _is_daylight, name_index] = raw;
    let offset = i64::from(i32::from_be_bytes([first, second, third, fourth]));
    if !OFFSET_RANGE.contains(&offset) {
        return None;
    }
    let name = names.get(usize::from(name_index)..)?;
    let end = name.iter().position(|&byte| byte == 0)?;
    let abbreviation = String::from_utf8(name.get(..end)?.to_vec()).ok()?;
    Some(Kind {
        offset,
        abbreviation,
    })
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
