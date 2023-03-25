//! Decodes data gradually from an audio file.

#![cfg_attr(
	docsrs,
	doc(cfg(all(
		any(feature = "mp3", feature = "ogg", feature = "flac", feature = "wav"),
		not(wasm32)
	)))
)]

mod data;
pub(crate) mod decoder;
mod handle;
mod settings;
mod sound;

pub use data::*;
pub use handle::*;
pub use settings::*;

use crate::{parameter::Value, tween::Tween, PlaybackRate, Volume};

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum Command {
	SetVolume(Value<Volume>, Tween),
	SetPlaybackRate(Value<PlaybackRate>, Tween),
	SetPanning(Value<f64>, Tween),
	Pause(Tween),
	Resume(Tween),
	Stop(Tween),
	SeekBy(f64),
	SeekTo(f64),
}
