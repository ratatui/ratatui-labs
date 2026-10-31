//! Per-line row counts with cheap positional edits.
//!
//! [`RowCounts`] stores one wrapped row count per source line in chunks of at most
//! [`CHUNK`] lines, with running line and row totals per chunk. Every operation touches one or two
//! chunks plus the per-chunk totals, so edits, inserts, and removals anywhere cost O(chunk + n /
//! chunk) instead of O(n), and lookups cost O(log n + chunk).

use std::ops::Range;

/// Most lines held by one chunk.
const CHUNK: usize = 512;

/// Wrapped row counts for a sequence of source lines.
#[derive(Debug, Clone, Default)]
pub(crate) struct RowCounts {
    /// Row counts, chunked in line order. No chunk is empty.
    chunks: Vec<Vec<usize>>,
    /// Lines before each chunk, plus the total line count at the end.
    line_starts: Vec<usize>,
    /// Rows before each chunk, plus the total row count at the end.
    row_starts: Vec<usize>,
}

impl RowCounts {
    /// Returns the number of lines.
    pub(crate) fn len(&self) -> usize {
        self.line_starts.last().copied().unwrap_or(0)
    }

    /// Returns the total number of rows.
    pub(crate) fn total(&self) -> usize {
        self.row_starts.last().copied().unwrap_or(0)
    }

    /// Returns the row count of `line`.
    pub(crate) fn get(&self, line: usize) -> usize {
        let (chunk, offset) = self.chunk_of(line);
        self.chunks[chunk][offset]
    }

    /// Returns the rows before `line`. `line` may equal [`Self::len`].
    pub(crate) fn rows_before(&self, line: usize) -> usize {
        if line >= self.len() {
            return self.total();
        }
        let (chunk, offset) = self.chunk_of(line);
        self.row_starts[chunk] + self.chunks[chunk][..offset].iter().sum::<usize>()
    }

    /// Returns the line containing `row` and the rows before that line.
    ///
    /// Lines with zero rows are skipped. `row` must be less than [`Self::total`].
    pub(crate) fn locate(&self, row: usize) -> (usize, usize) {
        let chunk = self.row_starts.partition_point(|&start| start <= row) - 1;
        let mut before = self.row_starts[chunk];
        for (offset, &count) in self.chunks[chunk].iter().enumerate() {
            if row < before + count {
                return (self.line_starts[chunk] + offset, before);
            }
            before += count;
        }
        unreachable!("row {row} is within chunk {chunk}")
    }

    /// Replaces the count of `line`.
    pub(crate) fn set(&mut self, line: usize, count: usize) {
        let (chunk, offset) = self.chunk_of(line);
        self.chunks[chunk][offset] = count;
        self.refresh_from(chunk);
    }

    /// Inserts `counts` before `line`. `line` may equal [`Self::len`].
    pub(crate) fn insert(&mut self, line: usize, counts: impl IntoIterator<Item = usize>) {
        let (chunk, offset) = if line >= self.len() {
            match self.chunks.last() {
                Some(last) if last.len() < CHUNK => (self.chunks.len() - 1, last.len()),
                _ => {
                    self.chunks.push(Vec::new());
                    (self.chunks.len() - 1, 0)
                }
            }
        } else {
            self.chunk_of(line)
        };
        let target = &mut self.chunks[chunk];
        let tail = target.split_off(offset);
        target.extend(counts);
        target.extend(tail);
        if target.is_empty() {
            self.chunks.remove(chunk);
        } else if target.len() > CHUNK {
            let pieces: Vec<Vec<usize>> = target.chunks(CHUNK).map(<[usize]>::to_vec).collect();
            self.chunks.splice(chunk..=chunk, pieces);
        }
        self.refresh_from(chunk);
    }

    /// Removes the counts of `lines`.
    pub(crate) fn remove(&mut self, lines: Range<usize>) {
        let end = lines.end.min(self.len());
        if lines.start >= end {
            return;
        }
        let (first, start) = self.chunk_of(lines.start);
        let mut remaining = end - lines.start;
        let mut chunk = first;
        let mut offset = start;
        while remaining > 0 {
            let target = &mut self.chunks[chunk];
            let take = remaining.min(target.len() - offset);
            target.drain(offset..offset + take);
            remaining -= take;
            chunk += 1;
            offset = 0;
        }
        self.chunks.retain(|chunk| !chunk.is_empty());
        // Merge the chunk that lost lines with its successor when both are small.
        let first = first.min(self.chunks.len().saturating_sub(1));
        if first + 1 < self.chunks.len()
            && self.chunks[first].len() + self.chunks[first + 1].len() <= CHUNK
        {
            let next = self.chunks.remove(first + 1);
            self.chunks[first].extend(next);
        }
        self.refresh_from(first);
    }

    /// Removes every count.
    pub(crate) fn clear(&mut self) {
        self.chunks.clear();
        self.line_starts.clear();
        self.row_starts.clear();
    }

    /// Returns the chunk holding `line` and its offset within the chunk.
    fn chunk_of(&self, line: usize) -> (usize, usize) {
        let chunk = self.line_starts.partition_point(|&start| start <= line) - 1;
        (chunk, line - self.line_starts[chunk])
    }

    /// Recomputes the running totals from chunk `first` onwards.
    fn refresh_from(&mut self, first: usize) {
        let first = first.min(self.chunks.len());
        self.line_starts.truncate(first);
        self.row_starts.truncate(first);
        let (mut lines, mut rows) = match first {
            0 => (0, 0),
            _ => (
                self.line_starts[first - 1] + self.chunks[first - 1].len(),
                self.row_starts[first - 1] + self.chunks[first - 1].iter().sum::<usize>(),
            ),
        };
        for chunk in &self.chunks[first..] {
            self.line_starts.push(lines);
            self.row_starts.push(rows);
            lines += chunk.len();
            rows += chunk.iter().sum::<usize>();
        }
        self.line_starts.push(lines);
        self.row_starts.push(rows);
    }
}

/// Unit tests for chunked row counts.
#[cfg(test)]
mod tests {
    use super::*;

    /// Checks every query against a plain vector through a mix of edits that cross chunk limits.
    #[test]
    fn matches_a_plain_vector() {
        let mut counts = RowCounts::default();
        let mut expected: Vec<usize> = Vec::new();
        let check = |counts: &RowCounts, expected: &[usize]| {
            assert_eq!(counts.len(), expected.len());
            assert_eq!(counts.total(), expected.iter().sum::<usize>());
            for line in 0..=expected.len() {
                assert_eq!(
                    counts.rows_before(line),
                    expected[..line].iter().sum::<usize>()
                );
            }
            for (line, &count) in expected.iter().enumerate() {
                assert_eq!(counts.get(line), count);
            }
            let mut row = 0;
            for (line, &count) in expected.iter().enumerate() {
                for _ in 0..count {
                    assert_eq!(counts.locate(row), (line, counts.rows_before(line)));
                    row += 1;
                }
            }
        };

        let value = |i: usize| (i * 7 + 3) % 4;
        counts.insert(0, (0..1300).map(value));
        expected.extend((0..1300).map(value));
        check(&counts, &expected);

        counts.insert(700, (0..600).map(|i| value(i + 1)));
        expected.splice(700..700, (0..600).map(|i| value(i + 1)));
        check(&counts, &expected);

        for line in [0, 511, 512, 1899] {
            counts.set(line, 9);
            expected[line] = 9;
        }
        check(&counts, &expected);

        counts.remove(100..1500);
        expected.drain(100..1500);
        check(&counts, &expected);

        counts.insert(counts.len(), [5, 0, 2]);
        expected.extend([5, 0, 2]);
        counts.remove(0..3);
        expected.drain(0..3);
        check(&counts, &expected);

        counts.clear();
        expected.clear();
        check(&counts, &expected);
    }
}
