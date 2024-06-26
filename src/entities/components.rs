use glam::DVec3;

#[derive(Debug)]
pub struct Orientation {
    yaw: f64,
    pitch: f64,

    pitch_min: f64,
    pitch_max: f64,
}

impl Orientation {
    pub fn new() -> Orientation {
        Orientation {
            yaw: 0.0,
            pitch: 0.0,

            pitch_min: 0.0,
            pitch_max: 0.0,
        }
    }

    pub fn new_with_values(yaw: f64, pitch: f64, pitch_min: f64, pitch_max: f64) -> Orientation {
        Orientation {
            yaw,
            pitch,
            pitch_min,
            pitch_max,
        }
    }

    pub fn set_yaw(&mut self, yaw: f64) {
        self.yaw = yaw;
    }

    pub fn set_pitch(&mut self, pitch: f64) {
        self.pitch = pitch;

        if self.pitch_min != 0.0 && self.pitch < self.pitch_min {
            self.pitch = self.pitch_min;
        }
        if self.pitch_max != 0.0 && self.pitch > self.pitch_max {
            self.pitch = self.pitch_max;
        }
    }

    pub fn set(&mut self, yaw: f64, pitch: f64) {
        self.set_yaw(yaw);
        self.set_pitch(pitch);
    }

    // Sets yaw and pitch to face in the direction of a provided vector
    pub fn set_by_look_vector(&mut self, dir: DVec3) {
        let r = (dir.x * dir.x + dir.y * dir.y + dir.z * dir.z).sqrt();
        let mut yaw = -dir.x.atan2(dir.z).to_degrees();
        if yaw < 0.0 {
            yaw += 360.0;
        }
        let pitch = -(dir.y / r).asin().to_degrees();
        self.yaw = yaw;
        self.pitch = pitch;
    }

    pub fn get_yaw(&self) -> f64 {
        self.yaw
    }

    pub fn get_pitch(&self) -> f64 {
        self.pitch
    }

    pub fn rotate(&mut self, yaw: f64, pitch: f64) {
        self.yaw += yaw;
        self.yaw %= 360.0;

        self.pitch += pitch;
        self.pitch %= 360.0;

        if self.pitch_min != 0.0 && self.pitch < self.pitch_min {
            self.pitch = self.pitch_min;
        }
        if self.pitch_max != 0.0 && self.pitch > self.pitch_max {
            self.pitch = self.pitch_max;
        }
    }

    /// Returns a 3-tuple for a unit vector in the direction of the yaw and pitch
    pub fn get_look_vector(&self) -> DVec3 {
        let x = -self.pitch.to_radians().cos() * self.yaw.to_radians().sin();
        let y = -self.pitch.to_radians().sin();
        let z = self.pitch.to_radians().cos() * self.yaw.to_radians().cos();
        DVec3::new(x, y, z)
    }

    pub fn get_min_pitch(&self) -> f64 {
        self.pitch_min
    }
    pub fn get_max_pitch(&self) -> f64 {
        self.pitch_max
    }

    pub fn set_min_pitch(&mut self, pitch_min: f64) {
        self.pitch_min = pitch_min;
    }
    pub fn set_max_pitch(&mut self, pitch_max: f64) {
        self.pitch_max = pitch_max;
    }

    pub fn get_rotations(&self) -> DVec3 {
        DVec3::new(self.yaw, self.pitch, 0.0)
    }
}
