use basedrop::Handle;
use ringbuf::Consumer;

use crate::{
	metronome::Metronome,
	mixer::{track::TrackInput, Mixer},
	parameter::Parameter,
	sequence::instance::SequenceInstance,
	sound::instance::{Instance, InstancePlaybackState},
	Frame,
};

use super::{command::Command, AudioManagerSettings};

pub struct Backend {
	sample_rate: u32,
	dt: f64,
	command_consumer: Consumer<Command>,
	instances: Vec<Instance>,
	metronomes: Vec<Metronome>,
	sequence_instances: Vec<SequenceInstance>,
	parameters: Vec<Parameter>,
	mixer: Mixer,
}

impl Backend {
	pub(crate) fn new(
		sample_rate: u32,
		command_consumer: Consumer<Command>,
		collector_handle: &Handle,
		settings: AudioManagerSettings,
	) -> Self {
		Self {
			sample_rate,
			dt: 1.0 / sample_rate as f64,
			command_consumer,
			instances: Vec::with_capacity(settings.num_instances),
			metronomes: Vec::with_capacity(settings.num_metronomes),
			sequence_instances: Vec::with_capacity(settings.num_sequences),
			parameters: Vec::with_capacity(settings.num_parameters),
			mixer: Mixer::new(collector_handle, settings.num_sub_tracks),
		}
	}

	pub(crate) fn main_track_input(&self) -> TrackInput {
		self.mixer.main_track().input().clone()
	}

	fn update_parameters(&mut self) {
		for parameter in &mut self.parameters {
			parameter.update(self.dt);
		}
	}

	fn update_metronomes(&mut self) {
		for metronome in &mut self.metronomes {
			metronome.update(self.dt);
		}
	}

	fn update_sequence_instances(&mut self) {
		let main_track_input = self.main_track_input();
		for sequence_instance in &mut self.sequence_instances {
			sequence_instance.update(self.dt, main_track_input.clone());
			for instance in sequence_instance.drain_instance_queue() {
				if self.instances.len() < self.instances.capacity() {
					self.instances.push(instance);
				}
			}
		}
		self.sequence_instances
			.retain(|instance| !instance.finished());
	}

	fn process_instances(&mut self) {
		let dt = self.dt;
		for instance in &mut self.instances {
			instance.process(dt);
		}
		self.instances
			.retain(|instance| instance.state() != InstancePlaybackState::Stopped);
	}

	pub fn process(&mut self) -> Frame {
		while let Some(command) = self.command_consumer.pop() {
			match command {
				Command::StartInstance { instance } => {
					if self.instances.len() < self.instances.capacity() {
						self.instances.push(instance);
					}
				}
				Command::StartSequenceInstance(mut instance) => {
					if self.sequence_instances.len() < self.sequence_instances.capacity() {
						instance.start();
						self.sequence_instances.push(instance);
					}
				}
				Command::AddMetronome(metronome) => {
					self.metronomes.push(metronome);
				}
				Command::AddParameter(parameter) => {
					self.parameters.push(parameter);
				}
				Command::AddSubTrack(sub_track) => self.mixer.add_sub_track(sub_track),
			}
		}

		self.update_parameters();
		self.update_metronomes();
		self.update_sequence_instances();
		self.process_instances();
		self.mixer.process(self.dt)
	}
}
