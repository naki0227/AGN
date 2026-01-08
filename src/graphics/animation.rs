use std::time::{Duration, Instant};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Easing {
    Linear,
    EaseInOut,
    Elastic,
}

impl Easing {
    pub fn apply(&self, t: f32) -> f32 {
        match self {
            Easing::Linear => t,
            Easing::EaseInOut => {
                // Cubic ease in-out
                if t < 0.5 {
                    4.0 * t * t * t
                } else {
                    1.0 - (-2.0 * t + 2.0).powi(3) / 2.0
                }
            },
            Easing::Elastic => {
                // Simplified elastic out
                if t == 0.0 { return 0.0; }
                if t == 1.0 { return 1.0; }
                let c4 = (2.0 * std::f32::consts::PI) / 3.0;
                2.0f32.powf(-10.0 * t) * ((t * 10.0 - 0.75) * c4).sin() + 1.0
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct Animation {
    pub target_id: String,
    pub property: String, // "shadow", "background", etc.
    pub start_value: f32, // Simplified to f32 for now (colors need mapping)
    pub end_value: f32,
    pub start_time: Instant,
    pub duration: Duration,
    pub easing: Easing,
}

pub struct AnimationController {
    animations: Vec<Animation>,
    values: HashMap<(String, String), f32>,
}

impl AnimationController {
    pub fn new() -> Self {
        Self {
            animations: Vec::new(),
            values: HashMap::new(),
        }
    }

    pub fn add_animation(&mut self, mut anim: Animation) {
        // Remove existing animation for same target+property
        self.animations.retain(|a| !(a.target_id == anim.target_id && a.property == anim.property));
        
        // Use current value as start_value
        let key = (anim.target_id.clone(), anim.property.clone());
        let current_val = self.values.get(&key).copied().unwrap_or(0.0);
        anim.start_value = current_val;
        
        self.animations.push(anim);
    }

    pub fn get_value(&self, target: &str, property: &str) -> Option<f32> {
        self.values.get(&(target.to_string(), property.to_string())).copied()
    }

    pub fn update(&mut self) -> Vec<(String, String, f32)> {
        let now = Instant::now();
        let mut updates = Vec::new();
        let mut finished_indices = Vec::new();

        for (i, anim) in self.animations.iter().enumerate() {
            let elapsed = now.duration_since(anim.start_time).as_secs_f32();
            let total = anim.duration.as_secs_f32();
            let t = (elapsed / total).clamp(0.0, 1.0);
            
            let eased_t = anim.easing.apply(t);
            let current_value = anim.start_value + (anim.end_value - anim.start_value) * eased_t;
            
            // Update persistent value
            self.values.insert((anim.target_id.clone(), anim.property.clone()), current_value);
            
            updates.push((anim.target_id.clone(), anim.property.clone(), current_value));
            
            if t >= 1.0 {
                finished_indices.push(i);
            }
        }

        // Cleanup finished animations (reverse order to keep indices valid)
        for index in finished_indices.into_iter().rev() {
            self.animations.remove(index);
        }

        updates
    }
}
