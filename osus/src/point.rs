use std::ops::{Add, Div, Mul, Neg, Sub};

#[derive(Clone, Copy, Debug, Default)]
pub struct Point {
	pub x: f64,
	pub y: f64,
}

impl Point {
	#[must_use]
	pub const fn new(x: f64, y: f64) -> Self {
		Self { x, y }
	}

	#[must_use]
	pub fn dot(self, rhs: Self) -> f64 {
		self.x.mul_add(rhs.x, self.y * rhs.y)
	}

	#[must_use]
	pub fn len(self) -> f64 {
		self.x.hypot(self.y)
	}

	#[must_use]
	pub fn normalized(self) -> Self {
		self / self.len()
	}
}

impl Neg for Point {
	type Output = Self;

	fn neg(self) -> Self::Output {
		Self { x: -self.x, y: -self.y }
	}
}

impl Add for Point {
	type Output = Self;

	fn add(self, rhs: Self) -> Self::Output {
		Self {
			x: self.x + rhs.x,
			y: self.y + rhs.y,
		}
	}
}

impl Sub for Point {
	type Output = Self;

	fn sub(self, rhs: Self) -> Self::Output {
		Self {
			x: self.x - rhs.x,
			y: self.y - rhs.y,
		}
	}
}

impl Mul<f64> for Point {
	type Output = Self;

	fn mul(self, rhs: f64) -> Self::Output {
		Self {
			x: self.x * rhs,
			y: self.y * rhs,
		}
	}
}

impl Div<f64> for Point {
	type Output = Self;

	fn div(self, rhs: f64) -> Self::Output {
		Self {
			x: self.x / rhs,
			y: self.y / rhs,
		}
	}
}
