//! Flow of a wizard (cloning, primer design …): steps, progress and allowed transitions.

/// State of a wizard with fixed steps.
#[derive(Clone, Debug, PartialEq)]
pub struct Wizard {
    steps: Vec<String>,
    current: usize,
    done: Vec<bool>,
}

impl Wizard {
    /// New wizard at the first step, nothing completed.
    pub fn new(steps: &[&str]) -> Self {
        Self {
            steps: steps.iter().map(|s| s.to_string()).collect(),
            current: 0,
            done: vec![false; steps.len()],
        }
    }

    /// Name of the current step (`None` without steps).
    pub fn current_step(&self) -> Option<&str> {
        self.steps.get(self.current).map(String::as_str)
    }

    /// Index of the current step.
    pub fn current_index(&self) -> usize {
        self.current
    }

    /// Marks the current step as completed (without steps: nothing).
    pub fn complete_current(&mut self) {
        if let Some(d) = self.done.get_mut(self.current) {
            *d = true;
        }
    }

    /// Move on to the next step; only if the current one is completed and is not the last. `true` if the step changed.
    #[allow(clippy::should_implement_trait)]
    pub fn next(&mut self) -> bool {
        let finished_here = self.done.get(self.current).copied().unwrap_or(false);
        if finished_here && self.current + 1 < self.steps.len() {
            self.current += 1;
            true
        } else {
            false
        }
    }

    /// One step back (the completed marks remain); `false` at the first step.
    pub fn back(&mut self) -> bool {
        if self.current > 0 {
            self.current -= 1;
            true
        } else {
            false
        }
    }

    /// Jumps to step `index` if it exists and all steps before it are completed. `true` if the step changed.
    pub fn go_to(&mut self, index: usize) -> bool {
        if index < self.steps.len() && self.done[..index].iter().all(|d| *d) {
            self.current = index;
            true
        } else {
            false
        }
    }

    /// Fraction of completed steps (0.0–1.0); 0.0 without steps.
    pub fn progress(&self) -> f64 {
        if self.steps.is_empty() {
            return 0.0;
        }
        self.done.iter().filter(|d| **d).count() as f64 / self.steps.len() as f64
    }

    /// `true` if there is at least one step and all are completed.
    pub fn is_finished(&self) -> bool {
        !self.steps.is_empty() && self.done.iter().all(|d| *d)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ablauf() {
        let mut w = Wizard::new(&["Vektor", "Insert", "Enzyme"]);
        assert_eq!(w.current_step(), Some("Vektor"));
        assert!(!w.next()); // step 0 not yet completed
        w.complete_current();
        assert!(w.next());
        assert_eq!(w.current_index(), 1);
        assert!(w.back());
        assert!(!w.back());
        assert!(!w.go_to(2)); // step 1 not completed
        assert!(w.go_to(1));
        w.complete_current();
        assert!(w.go_to(2));
        assert!((w.progress() - 2.0 / 3.0).abs() < 1e-9);
        assert!(!w.is_finished());
        w.complete_current();
        assert!(w.is_finished());
        assert!(!w.next()); // last step
        assert!(!w.go_to(3)); // does not exist
    }

    #[test]
    fn leer() {
        let mut w = Wizard::new(&[]);
        assert_eq!(w.current_step(), None);
        assert_eq!(w.progress(), 0.0);
        assert!(!w.is_finished());
        assert!(!w.next());
        w.complete_current();
    }
}
