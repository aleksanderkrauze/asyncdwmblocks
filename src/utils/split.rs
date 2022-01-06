pub struct SplitAtRN<'a> {
    buff: &'a [u8],
    was_last_r: bool,
}

impl<'a> SplitAtRN<'a> {
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
        for i in 0..self.buff.len() {
            match self.buff[i] {
                b'\r' => self.was_last_r = true,
                b'\n' => {
                    if self.was_last_r {
                        // Reset r flag
                        self.was_last_r = false;

                        let (left, right) = self.buff.split_at(i);
                        self.buff = &right[1..right.len()];

                        let left = &left[0..(left.len() - 1)];
                        return Some(left);
                    }
                    // Reset r flag
                    self.was_last_r = false;
                }
                // Reset r flag
                _ => self.was_last_r = false,
            }
        }

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
