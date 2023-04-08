// Code is based on https://cytomic.com/files/dsp/SvfLinearTrapOptimised2.pdf

mod builder;
mod handle;

pub use builder::*;
pub use handle::*;

use ringbuf::HeapConsumer;

use std::f32::consts::PI;

use crate::{
	clock::clock_info::ClockInfoProvider,
	dsp::Frame,
	modulator::value_provider::ModulatorValueProvider,
	parameter::{Parameter, Value},
	tween::Tween,
};

use super::Effect;

const MIN_Q: f32 = 0.01;

struct EqFilter {
	command_consumer: HeapConsumer<Command>,
	kind: EqFilterKind,
	frequency: Parameter<f32>,
	gain: Parameter<f32>,
	q: Parameter<f32>,
	ic1eq: Frame,
	ic2eq: Frame,
}

impl EqFilter {
	fn new(builder: EqFilterBuilder, command_consumer: HeapConsumer<Command>) -> Self {
		Self {
			command_consumer,
			kind: builder.kind,
			frequency: Parameter::new(builder.frequency, 500.0),
			gain: Parameter::new(builder.gain, 0.0),
			q: Parameter::new(builder.q, 1.0),
			ic1eq: Frame::ZERO,
			ic2eq: Frame::ZERO,
		}
	}

	fn calculate_coefficients(&self, dt: f64) -> Coefficients {
		// In my testing, the filter goes unstable when the frequency exceeds half the sample rate,
		// so I'm clamping this value to 0.5
		let relative_frequency = (self.frequency.value() * dt as f32).clamp(0.0, 0.5);
		let q = self.q.value().max(MIN_Q);
		match self.kind {
			EqFilterKind::Bell => {
				let a = 10.0f32.powf(self.gain.value() / 40.0);
				let g = (PI * relative_frequency).tan();
				let k = 1.0 / (q * a);
				let a1 = 1.0 / (1.0 + g * (g + k));
				let a2 = g * a1;
				let a3 = g * a2;
				let m0 = 1.0;
				let m1 = k * (a * a - 1.0);
				let m2 = 0.0;
				Coefficients {
					a1,
					a2,
					a3,
					m0,
					m1,
					m2,
				}
			}
			EqFilterKind::LowShelf => {
				let a = 10.0f32.powf(self.gain.value() / 40.0);
				let g = (PI * relative_frequency).tan() / a.sqrt();
				let k = 1.0 / q;
				let a1 = 1.0 / (1.0 + g * (g + k));
				let a2 = g * a1;
				let a3 = g * a2;
				let m0 = 1.0;
				let m1 = k * (a - 1.0);
				let m2 = a * a - 1.0;
				Coefficients {
					a1,
					a2,
					a3,
					m0,
					m1,
					m2,
				}
			}
			EqFilterKind::HighShelf => {
				let a = 10.0f32.powf(self.gain.value() / 40.0);
				let g = (PI * relative_frequency).tan() * a.sqrt();
				let k = 1.0 / q;
				let a1 = 1.0 / (1.0 + g * (g + k));
				let a2 = g * a1;
				let a3 = g * a2;
				let m0 = a * a;
				let m1 = k * (1.0 - a) * a;
				let m2 = 1.0 - a * a;
				Coefficients {
					a1,
					a2,
					a3,
					m0,
					m1,
					m2,
				}
			}
		}
	}
}

impl Effect for EqFilter {
	fn on_start_processing(&mut self) {
		while let Some(command) = self.command_consumer.pop() {
			match command {
				Command::SetFrequency(target, tween) => self.frequency.set(target, tween),
				Command::SetGain(target, tween) => self.gain.set(target, tween),
				Command::SetQ(target, tween) => self.q.set(target, tween),
			}
		}
	}

	fn process(
		&mut self,
		input: Frame,
		dt: f64,
		clock_info_provider: &ClockInfoProvider,
		modulator_value_provider: &ModulatorValueProvider,
	) -> Frame {
		self.frequency
			.update(dt, clock_info_provider, modulator_value_provider);
		self.gain
			.update(dt, clock_info_provider, modulator_value_provider);
		self.q
			.update(dt, clock_info_provider, modulator_value_provider);
		let Coefficients {
			a1,
			a2,
			a3,
			m0,
			m1,
			m2,
		} = self.calculate_coefficients(dt);
		let v3 = input - self.ic2eq;
		let v1 = self.ic1eq * a1 + v3 * a2;
		let v2 = self.ic2eq + self.ic1eq * a2 + v3 * a3;
		self.ic1eq = v1 * 2.0 - self.ic1eq;
		self.ic2eq = v2 * 2.0 - self.ic2eq;
		input * m0 + v1 * m1 + v2 * m2
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EqFilterKind {
	Bell,
	LowShelf,
	HighShelf,
}

struct Coefficients {
	a1: f32,
	a2: f32,
	a3: f32,
	m0: f32,
	m1: f32,
	m2: f32,
}

enum Command {
	SetFrequency(Value<f32>, Tween),
	SetGain(Value<f32>, Tween),
	SetQ(Value<f32>, Tween),
}
