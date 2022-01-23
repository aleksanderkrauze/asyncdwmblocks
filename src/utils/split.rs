//! This module defines [SplitAtRN]. A struct that allows to split
//! stream of bytes at `b"\r\n"`.

/// Splits stream of bytes at `b"\r\n"`.
///
/// More precisely it returns (as an `Iterator`) a list of
/// sub slices of given `&[u8]` divided **and ended** by `b"\r\n"` sequence.
/// It therefore does not behave *exactly* as a standard split from for
/// ex. standard library. See example.
///
/// # Example
/// ```
/// use asyncdwmblocks::utils::SplitAtRN;
///
/// let bytes = b"Robin Hood\r\nLittle John\r\nSherif of Notthingham";
/// let mut sherwood_company = SplitAtRN::new(bytes);
///
/// assert_eq!(sherwood_company.next(), Some(b"Robin Hood".as_slice()));
/// assert_eq!(sherwood_company.next(), Some(b"Little John".as_slice()));
/// assert_eq!(sherwood_company.next(), None); // bytes were not ended by `b"\r\n"`!
/// ```
#[derive(Debug, PartialEq, Clone)]
pub struct SplitAtRN<'a> {
    buff: &'a [u8],
    was_last_r: bool,
}

impl<'a> SplitAtRN<'a> {
    /// Creates new `SplitAtRN` from given bytes slice.
    pub fn new(buff: &'a [u8]) -> Self {
        Self {
            buff,
            was_last_r: false,
        }
    }
}

impl<'a> Iterator for SplitAtRN<'a> {
    type Item = &'a [u8];
    fn next(&mut self) -> Option<Self::Item> {
        // Iterate over all bytes of **current** buffer. When buffer
        // is repositioned we still need to look at all of it's bytes
        // (as we haven't seen them before).
        for i in 0..self.buff.len() {
            // If current byte is b'\r' then we set `was_last_r` flag to true.
            // In any other case we must reset this flag to false.
            // When we encounter b'\n' and `was_last_r` flag is true then we
            // have found splitting sequence and perform splitting, shifting
            // slices and return founded sub-slice.
            match self.buff[i] {
                b'\r' => self.was_last_r = true,
                b'\n' => {
                    let was_last_r = self.was_last_r;
                    self.was_last_r = false;

                    if was_last_r {
                        let (left, right) = self.buff.split_at(i);
                        self.buff = &right[1..right.len()];

                        let left = &left[0..(left.len() - 1)];
                        return Some(left);
                    }
                }
                _ => self.was_last_r = false,
            }
        }

        // We have looked at all bytes, end this iterator.
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_at_rn_no_rn() {
        let mut data = SplitAtRN::new(b"test");
        assert_eq!(data.next(), None);
    }

    #[test]
    fn split_at_rn_trailing() {
        let mut data = SplitAtRN::new(b"test1\r\ntest2");
        assert_eq!(data.next(), Some(b"test1".as_slice()));
        assert_eq!(data.next(), None);
    }

    #[test]
    fn split_at_rn_trailing_r() {
        let mut data = SplitAtRN::new(b"X\r\ntest\r");
        assert_eq!(data.next(), Some(b"X".as_slice()));
        assert_eq!(data.next(), None);
    }

    #[test]
    fn split_at_rn_trailing_n() {
        let mut data = SplitAtRN::new(b"X\r\ntest\n");
        assert_eq!(data.next(), Some(b"X".as_slice()));
        assert_eq!(data.next(), None);
    }

    #[test]
    fn split_at_rn_r_before_rn() {
        let mut data = SplitAtRN::new(b"X\r\r\n");
        assert_eq!(data.next(), Some(b"X\r".as_slice()));
        assert_eq!(data.next(), None);
    }

    #[test]
    fn split_at_rn_n_after_rn() {
        let mut data = SplitAtRN::new(b"X\r\n\n");
        assert_eq!(data.next(), Some(b"X".as_slice()));
        assert_eq!(data.next(), None);
    }

    #[test]
    fn split_at_rn_empty() {
        let mut data = SplitAtRN::new(b"\r\n");
        assert_eq!(data.next(), Some(b"".as_slice()));
        assert_eq!(data.next(), None);
    }

    #[test]
    fn split_at_rn_single() {
        let mut data = SplitAtRN::new(b"test\r\n");
        assert_eq!(data.next(), Some(b"test".as_slice()));
        assert_eq!(data.next(), None);
    }

    #[test]
    fn split_at_rn_multiple() {
        let mut data = SplitAtRN::new(b"test1\r\ntest2\r\ntest3\r\n");
        assert_eq!(data.next(), Some(b"test1".as_slice()));
        assert_eq!(data.next(), Some(b"test2".as_slice()));
        assert_eq!(data.next(), Some(b"test3".as_slice()));
        assert_eq!(data.next(), None);
    }

    #[test]
    fn split_at_rn_long_and_short() {
        let mut data = SplitAtRN::new(b"This is a very long block.\r\nshort\r\n");
        assert_eq!(data.next(), Some(b"This is a very long block.".as_slice()));
        assert_eq!(data.next(), Some(b"short".as_slice()));
        assert_eq!(data.next(), None);
    }

    #[test]
    fn split_at_rn_adjecent_rn() {
        let mut data = SplitAtRN::new(b"test1\r\n\r\n");
        assert_eq!(data.next(), Some(b"test1".as_slice()));
        assert_eq!(data.next(), Some(b"".as_slice()));
        assert_eq!(data.next(), None);
    }

    #[test]
    fn split_at_rn_alone_r() {
        let mut data = SplitAtRN::new(b"ab\rc\r\nABC\r\n");
        assert_eq!(data.next(), Some(b"ab\rc".as_slice()));
        assert_eq!(data.next(), Some(b"ABC".as_slice()));
        assert_eq!(data.next(), None);
    }

    #[test]
    fn split_at_rn_alone_n() {
        let mut data = SplitAtRN::new(b"ab\nc\r\nABC\r\n");
        assert_eq!(data.next(), Some(b"ab\nc".as_slice()));
        assert_eq!(data.next(), Some(b"ABC".as_slice()));
        assert_eq!(data.next(), None);
    }
}
