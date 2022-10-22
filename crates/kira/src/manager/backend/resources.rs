pub(crate) mod clocks;
pub(crate) mod mixer;
pub(crate) mod sounds;

use atomic_arena::Controller;
use ringbuf::{HeapConsumer, HeapProducer, HeapRb};

use crate::{
	clock::Clock,
	manager::settings::Capacities,
	sound::Sound,
	track::{Track, TrackBuilder},
};

use self::{clocks::Clocks, mixer::Mixer, sounds::Sounds};

pub(crate) struct UnusedResourceProducers {
	pub sound: HeapProducer<Box<dyn Sound>>,
	pub sub_track: HeapProducer<Track>,
	pub clock: HeapProducer<Clock>,
}

pub(crate) struct UnusedResourceConsumers {
	pub sound: HeapConsumer<Box<dyn Sound>>,
	pub sub_track: HeapConsumer<Track>,
	pub clock: HeapConsumer<Clock>,
}

pub(crate) fn create_unused_resource_channels(
	capacities: Capacities,
) -> (UnusedResourceProducers, UnusedResourceConsumers) {
	let (unused_sound_producer, unused_sound_consumer) =
		HeapRb::new(capacities.sound_capacity).split();
	let (unused_sub_track_producer, unused_sub_track_consumer) =
		HeapRb::new(capacities.sub_track_capacity).split();
	let (unused_clock_producer, unused_clock_consumer) =
		HeapRb::new(capacities.clock_capacity).split();
	(
		UnusedResourceProducers {
			sound: unused_sound_producer,
			sub_track: unused_sub_track_producer,
			clock: unused_clock_producer,
		},
		UnusedResourceConsumers {
			sound: unused_sound_consumer,
			sub_track: unused_sub_track_consumer,
			clock: unused_clock_consumer,
		},
	)
}

pub(crate) struct Resources {
	pub sounds: Sounds,
	pub mixer: Mixer,
	pub clocks: Clocks,
}

pub(crate) struct ResourceControllers {
	pub sound_controller: Controller,
	pub sub_track_controller: Controller,
	pub clock_controller: Controller,
}

pub(crate) fn create_resources(
	capacities: Capacities,
	main_track_builder: TrackBuilder,
	unused_resource_producers: UnusedResourceProducers,
	sample_rate: u32,
) -> (Resources, ResourceControllers) {
	let sounds = Sounds::new(capacities.sound_capacity, unused_resource_producers.sound);
	let sound_controller = sounds.controller();
	let mixer = Mixer::new(
		capacities.sub_track_capacity,
		unused_resource_producers.sub_track,
		sample_rate,
		main_track_builder,
	);
	let sub_track_controller = mixer.sub_track_controller();
	let clocks = Clocks::new(capacities.clock_capacity, unused_resource_producers.clock);
	let clock_controller = clocks.controller();
	(
		Resources {
			sounds,
			mixer,
			clocks,
		},
		ResourceControllers {
			sound_controller,
			sub_track_controller,
			clock_controller,
		},
	)
}
