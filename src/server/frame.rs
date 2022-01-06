use crate::block::BlockRunMode;
use crate::statusbar::BlockRefreshMessage;
use crate::utils::SplitAtRN;

#[derive(Debug, PartialEq, Clone)]
pub enum Frame {
    Message(BlockRefreshMessage),
    Error,
}

impl From<&[u8]> for Frame {
    fn from(data: &[u8]) -> Self {
        let data = match String::from_utf8(Vec::from(data)) {
            Ok(data) => data,
            Err(_) => return Frame::Error,
        };

        let data = data.split_whitespace().collect::<Vec<_>>();

        match data.len() {
            2 => {
                if data[0].to_uppercase() == "REFRESH" {
                    Frame::Message(BlockRefreshMessage::new(
                        String::from(data[1]),
                        BlockRunMode::Normal,
                    ))
                } else {
                    Frame::Error
                }
            }
            3 => {
                let num = data[1].parse::<u8>();
                if data[0].to_uppercase() == "BUTTON" {
                    if let Ok(num) = num {
                        Frame::Message(BlockRefreshMessage::new(
                            String::from(data[2]),
                            BlockRunMode::Button(num),
                        ))
                    } else {
                        Frame::Error
                    }
                } else {
                    Frame::Error
                }
            }
            _ => Frame::Error,
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct Frames {
    frames: Vec<Frame>,
}

impl IntoIterator for Frames {
    type Item = Frame;
    type IntoIter = std::vec::IntoIter<Self::Item>;
    fn into_iter(self) -> Self::IntoIter {
        self.frames.into_iter()
    }
}

impl FromIterator<Frame> for Frames {
    fn from_iter<I: IntoIterator<Item = Frame>>(iter: I) -> Self {
        Self {
            frames: iter.into_iter().collect(),
        }
    }
}

impl From<&[u8]> for Frames {
    fn from(data: &[u8]) -> Self {
        SplitAtRN::new(data).map(Frame::from).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_decoding_empty() {
        let frame = Frame::from(b"".as_slice());
        assert_eq!(frame, Frame::Error);
    }

    #[test]
    fn frame_decoding_empty_whitespaces() {
        let frame = Frame::from(b" \t\t   ".as_slice());
        assert_eq!(frame, Frame::Error);
    }

    #[test]
    fn frame_decoding_invalid() {
        let frame1 = Frame::from(b"Invalid_frame".as_slice());
        let frame2 = Frame::from(b"Invalid frame".as_slice());
        let frame3 = Frame::from(b"block_id REFRESH".as_slice());
        let frame4 = Frame::from(b"REFRESH 3 my_block".as_slice());
        let frame5 = Frame::from(b"REFRESH block1 block2".as_slice());
        let frame6 = Frame::from(b"BuTN 5 blockID=1".as_slice());
        let frame7 = Frame::from(b"BUTTON block 1".as_slice());
        let frame8 = Frame::from(b"BUTTON 1 block1 extra".as_slice());

        assert_eq!(frame1, Frame::Error);
        assert_eq!(frame2, Frame::Error);
        assert_eq!(frame3, Frame::Error);
        assert_eq!(frame4, Frame::Error);
        assert_eq!(frame5, Frame::Error);
        assert_eq!(frame6, Frame::Error);
        assert_eq!(frame7, Frame::Error);
        assert_eq!(frame8, Frame::Error);
    }

    #[test]
    fn frame_decoding_invalid_utf8() {
        let frame = Frame::from(b"REFRESH\xf0\x90\x28\xbc block_id".as_slice());
        assert_eq!(frame, Frame::Error);
    }

    #[test]
    fn frame_decoding_refresh() {
        let frame = Frame::from(b"refresh block1".as_slice());
        assert_eq!(
            frame,
            Frame::Message(BlockRefreshMessage::new(
                "block1".into(),
                BlockRunMode::Normal
            ))
        );
    }

    #[test]
    #[allow(non_snake_case)]
    fn frame_decoding_REFRESH() {
        let frame = Frame::from(b"REFRESH block1".as_slice());
        assert_eq!(
            frame,
            Frame::Message(BlockRefreshMessage::new(
                "block1".into(),
                BlockRunMode::Normal
            ))
        );
    }

    #[test]
    fn frame_decoding_refresh_different_cases() {
        let frame = Frame::from(b"rEFrEsH block1".as_slice());
        assert_eq!(
            frame,
            Frame::Message(BlockRefreshMessage::new(
                "block1".into(),
                BlockRunMode::Normal
            ))
        );
    }

    #[test]
    fn frame_decoding_refresh_extra_whitespaces() {
        let frame1 = Frame::from(b"REFRESH   block1 ".as_slice());
        let frame2 = Frame::from(b"REFRESH\tblock2".as_slice());
        let frame3 = Frame::from(b"REFRESH \t block3 \t".as_slice());
        let frame4 = Frame::from(b"REFRESH block4   ".as_slice());

        assert_eq!(
            frame1,
            Frame::Message(BlockRefreshMessage::new(
                "block1".into(),
                BlockRunMode::Normal
            ))
        );
        assert_eq!(
            frame2,
            Frame::Message(BlockRefreshMessage::new(
                "block2".into(),
                BlockRunMode::Normal
            ))
        );
        assert_eq!(
            frame3,
            Frame::Message(BlockRefreshMessage::new(
                "block3".into(),
                BlockRunMode::Normal
            ))
        );
        assert_eq!(
            frame4,
            Frame::Message(BlockRefreshMessage::new(
                "block4".into(),
                BlockRunMode::Normal
            ))
        );
    }

    #[test]
    fn frame_decoding_button() {
        let frame = Frame::from(b"button 1 block1".as_slice());
        assert_eq!(
            frame,
            Frame::Message(BlockRefreshMessage::new(
                "block1".into(),
                BlockRunMode::Button(1)
            ))
        );
    }

    #[test]
    #[allow(non_snake_case)]
    fn frame_decoding_BUTTON() {
        let frame = Frame::from(b"BUTTON 1 block1".as_slice());
        assert_eq!(
            frame,
            Frame::Message(BlockRefreshMessage::new(
                "block1".into(),
                BlockRunMode::Button(1)
            ))
        );
    }

    #[test]
    fn frame_decoding_button_different_cases() {
        let frame = Frame::from(b"BuTTon 1 block1".as_slice());
        assert_eq!(
            frame,
            Frame::Message(BlockRefreshMessage::new(
                "block1".into(),
                BlockRunMode::Button(1)
            ))
        );
    }

    #[test]
    fn frame_decoding_button_extra_whitespaces() {
        let frame1 = Frame::from(b"BUTTON  1  block1 ".as_slice());
        let frame2 = Frame::from(b"BUTTON\t2\tblock2".as_slice());
        let frame3 = Frame::from(b"BUTTON   3 block3   ".as_slice());
        let frame4 = Frame::from(b"BUTTON \t4  block4\t".as_slice());
        let frame5 = Frame::from(b"BUTTON \t 5\t\tblock5 \t ".as_slice());

        assert_eq!(
            frame1,
            Frame::Message(BlockRefreshMessage::new(
                "block1".into(),
                BlockRunMode::Button(1)
            ))
        );
        assert_eq!(
            frame2,
            Frame::Message(BlockRefreshMessage::new(
                "block2".into(),
                BlockRunMode::Button(2)
            ))
        );
        assert_eq!(
            frame3,
            Frame::Message(BlockRefreshMessage::new(
                "block3".into(),
                BlockRunMode::Button(3)
            ))
        );
        assert_eq!(
            frame4,
            Frame::Message(BlockRefreshMessage::new(
                "block4".into(),
                BlockRunMode::Button(4)
            ))
        );
        assert_eq!(
            frame5,
            Frame::Message(BlockRefreshMessage::new(
                "block5".into(),
                BlockRunMode::Button(5)
            ))
        );
    }

    #[test]
    fn frame_decoding_button_wrong_number() {
        let frame1 = Frame::from(b"BUTTON 1024 block1".as_slice());
        let frame2 = Frame::from(b"BUTTON A31 block1".as_slice());

        assert_eq!(frame1, Frame::Error);
        assert_eq!(frame2, Frame::Error);
    }

    #[test]
    fn frames() {
        let data = b"REFRESH temperature\r\nREFRESH volume\r\nBUTTON 1 battery\r\nREFRESH cpu\r\n";
        let frames = Frames::from(data.as_slice());

        assert_eq!(
            frames.frames,
            vec![
                Frame::Message(BlockRefreshMessage::new(
                    String::from("temperature"),
                    BlockRunMode::Normal
                )),
                Frame::Message(BlockRefreshMessage::new(
                    String::from("volume"),
                    BlockRunMode::Normal
                )),
                Frame::Message(BlockRefreshMessage::new(
                    String::from("battery"),
                    BlockRunMode::Button(1)
                )),
                Frame::Message(BlockRefreshMessage::new(
                    String::from("cpu"),
                    BlockRunMode::Normal
                ))
            ]
        );
    }
}
