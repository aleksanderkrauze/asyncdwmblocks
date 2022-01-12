//! This module defines framing model for translating
//! from/to byte stream to/from [`BlockRefreshMessage`]s.
//!
//! This module defines two types: [Frame] and [Frames].
//!
//! `Frame` is a single unit of translation. It represents
//! either a message ([`BlockRefreshMessage`]) or an `Error`
//! (which means that decoding failed). It is more useful
//! when used in context of `Frames`.
//!
//! `Frames` is a collection of `Frame`s. It implements both
//! [FromIterator] and [IntoIterator]. It can create collection
//! for `Frame`s from byte stream (by dividing it into blocks ended by `b"\r\n"`),
//! and can produce byte stream back from list of `Frame`s. See
//! examples for exemplary usage of them.
//!
//! # Decoding
//!
//! This example shows how byte stream could be decoded and interpreted
//! as a list of `BlockRefreshMessage`s. It is used in [`Server`](super::Server)s.
//!
//! ```
//! use asyncdwmblocks::ipc::frame::{Frames, Frame};
//!
//! # fn main() {
//! let stream = b"...";
//! let frames = Frames::from(stream.as_slice());
//! for frame in frames {
//!     match frame {
//!         Frame::Message(msg) => {
//!             // send interpreted message somewhere
//!         }
//!         Frame::Error => {
//!             // stream contained error, handle it or ignore
//!         }
//!     }
//! }
//! # }
//! ```
//!
//! # Encoding
//!
//! This example shows how list of `BlockRefreshMessage`s can be
//! encoded into byte stream. It is used in [`Notifier`](super::Notifier)s.
//!
//! ```
//! use asyncdwmblocks::statusbar::BlockRefreshMessage;
//! use asyncdwmblocks::block::BlockRunMode;
//! use asyncdwmblocks::ipc::frame::{Frames, Frame};
//!
//! # fn main() {
//! let messages = vec![
//!     BlockRefreshMessage::new(String::from("battery"), BlockRunMode::Normal),
//!     BlockRefreshMessage::new(String::from("backlight"), BlockRunMode::Button(1)),
//! ];
//! let frames: Frames = messages.into_iter().map(Frame::from).collect();
//! let stream: Vec<u8> = frames.encode(); // Send this stream somewhere
//! # }
//! ```

use crate::block::BlockRunMode;
use crate::statusbar::BlockRefreshMessage;
use crate::utils::SplitAtRN;

/// This enum defines single unit of translation.
///
/// `Frame` can either hold a message, or (when decoding)
/// and `Error` variant (which indicates that translation failed).
/// It can be created either from `&[u8]` (decoding) or from
/// `BlockRefreshMessage` (to be later encoded). In both cases it is
/// done by implementing [`From`] trait.
#[derive(Debug, PartialEq, Clone)]
pub enum Frame {
    /// This variant holds decoded/passed message.
    Message(BlockRefreshMessage),
    /// This variant indicates error while decoding.
    Error,
}

impl Frame {
    /// Encodes `Frame` into `Vec<u8>`.
    pub fn encode(&self) -> Vec<u8> {
        match self {
            Frame::Message(msg) => {
                let s = match msg {
                    BlockRefreshMessage {
                        name,
                        mode: BlockRunMode::Normal,
                    } => {
                        format!("REFRESH {}\r\n", name)
                    }
                    BlockRefreshMessage {
                        name,
                        mode: BlockRunMode::Button(b),
                    } => {
                        format!("BUTTON {} {}\r\n", b, name)
                    }
                };
                Vec::from(s.as_bytes())
            }
            Frame::Error => Vec::new(),
        }
    }
}

/// Creates `Frame` from byte stream. Used in decoding.
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

/// Creates `Frame` from `BlockRefreshMessage`. Used in encoding.
impl From<BlockRefreshMessage> for Frame {
    fn from(msg: BlockRefreshMessage) -> Self {
        Self::Message(msg)
    }
}

/// This struct represents a collection of `Frame`s.
///
/// It implements both `From<&u8>` and `FromIterator<Frame>`
/// which can be used to decode and encode frames respectively.
/// It also implements `IntoIterator` to allow easily iterating
/// over contained `Frame`s.
#[derive(Debug, PartialEq, Clone)]
pub struct Frames {
    frames: Vec<Frame>,
}

impl Frames {
    /// Encodes `Frames` into `Vec<u8>`.
    pub fn encode(&self) -> Vec<u8> {
        self.frames
            .iter()
            .map(|f| f.encode())
            .reduce(|mut acc, mut f| {
                acc.append(&mut f);
                acc
            })
            .unwrap_or_default()
    }
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
    fn frame_decode_empty() {
        let frame = Frame::from(b"".as_slice());
        assert_eq!(frame, Frame::Error);
    }

    #[test]
    fn frame_decode_empty_whitespaces() {
        let frame = Frame::from(b" \t\t   ".as_slice());
        assert_eq!(frame, Frame::Error);
    }

    #[test]
    fn frame_decode_invalid() {
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
    fn frame_decode_invalid_utf8() {
        let frame = Frame::from(b"REFRESH\xf0\x90\x28\xbc block_id".as_slice());
        assert_eq!(frame, Frame::Error);
    }

    #[test]
    fn frame_decode_refresh() {
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
    fn frame_decode_REFRESH() {
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
    fn frame_decode_refresh_different_cases() {
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
    fn frame_decode_refresh_extra_whitespaces() {
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
    fn frame_decode_button() {
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
    fn frame_decode_BUTTON() {
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
    fn frame_decode_button_different_cases() {
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
    fn frame_decode_button_extra_whitespaces() {
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
    fn frame_decode_button_wrong_number() {
        let frame1 = Frame::from(b"BUTTON 1024 block1".as_slice());
        let frame2 = Frame::from(b"BUTTON A31 block1".as_slice());

        assert_eq!(frame1, Frame::Error);
        assert_eq!(frame2, Frame::Error);
    }

    #[test]
    fn frame_encode() {
        let empty = Frame::Error;
        let normal = Frame::Message(BlockRefreshMessage::new(
            String::from("date"),
            BlockRunMode::Normal,
        ));
        let button1 = Frame::Message(BlockRefreshMessage::new(
            String::from("battery"),
            BlockRunMode::Button(1),
        ));
        let button2 = Frame::Message(BlockRefreshMessage::new(
            String::from("backlight"),
            BlockRunMode::Button(2),
        ));

        assert_eq!(empty.encode(), Vec::<u8>::new());
        assert_eq!(normal.encode(), Vec::from("REFRESH date\r\n".as_bytes()));
        assert_eq!(
            button1.encode(),
            Vec::from("BUTTON 1 battery\r\n".as_bytes())
        );
        assert_eq!(
            button2.encode(),
            Vec::from("BUTTON 2 backlight\r\n".as_bytes())
        );
    }

    #[test]
    fn frames_decode() {
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

    #[test]
    fn frames_encode() {
        let frames = vec![
            Frame::Message(BlockRefreshMessage::new(
                String::from("date"),
                BlockRunMode::Normal,
            )),
            Frame::Message(BlockRefreshMessage::new(
                String::from("battery"),
                BlockRunMode::Button(1),
            )),
            Frame::Message(BlockRefreshMessage::new(
                String::from("backlight"),
                BlockRunMode::Button(2),
            )),
        ];
        let frames = Frames::from_iter(frames);

        assert_eq!(
            frames.encode(),
            Vec::from("REFRESH date\r\nBUTTON 1 battery\r\nBUTTON 2 backlight\r\n".as_bytes())
        );
    }
}
