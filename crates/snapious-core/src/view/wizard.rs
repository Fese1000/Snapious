//! Ablauf eines Assistenten (Klonierung, Primerdesign …): Schritte, Fortschritt und erlaubte Wechsel.

/// Zustand eines Assistenten mit festen Schritten.
#[derive(Clone, Debug, PartialEq)]
pub struct Wizard {
    steps: Vec<String>,
    current: usize,
    done: Vec<bool>,
}

impl Wizard {
    /// Neuer Assistent am ersten Schritt, nichts erledigt.
    pub fn new(steps: &[&str]) -> Self {
        Self {
            steps: steps.iter().map(|s| s.to_string()).collect(),
            current: 0,
            done: vec![false; steps.len()],
        }
    }

    /// Name des aktuellen Schritts (`None` ohne Schritte).
    pub fn current_step(&self) -> Option<&str> {
        self.steps.get(self.current).map(String::as_str)
    }

    /// Index des aktuellen Schritts.
    pub fn current_index(&self) -> usize {
        self.current
    }

    /// Markiert den aktuellen Schritt als erledigt (ohne Schritte: nichts).
    pub fn complete_current(&mut self) {
        if let Some(d) = self.done.get_mut(self.current) {
            *d = true;
        }
    }

    /// Weiter zum nächsten Schritt; nur wenn der aktuelle erledigt und nicht der letzte ist. `true`, wenn gewechselt wurde.
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

    /// Einen Schritt zurück (die Erledigt-Markierungen bleiben); `false` am ersten Schritt.
    pub fn back(&mut self) -> bool {
        if self.current > 0 {
            self.current -= 1;
            true
        } else {
            false
        }
    }

    /// Springt zum Schritt `index`, wenn er existiert und alle Schritte davor erledigt sind. `true`, wenn gewechselt wurde.
    pub fn go_to(&mut self, index: usize) -> bool {
        if index < self.steps.len() && self.done[..index].iter().all(|d| *d) {
            self.current = index;
            true
        } else {
            false
        }
    }

    /// Anteil erledigter Schritte (0.0–1.0); ohne Schritte 0.0.
    pub fn progress(&self) -> f64 {
        if self.steps.is_empty() {
            return 0.0;
        }
        self.done.iter().filter(|d| **d).count() as f64 / self.steps.len() as f64
    }

    /// `true`, wenn es mindestens einen Schritt gibt und alle erledigt sind.
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
        assert!(!w.next()); // Schritt 0 noch nicht erledigt
        w.complete_current();
        assert!(w.next());
        assert_eq!(w.current_index(), 1);
        assert!(w.back());
        assert!(!w.back());
        assert!(!w.go_to(2)); // Schritt 1 nicht erledigt
        assert!(w.go_to(1));
        w.complete_current();
        assert!(w.go_to(2));
        assert!((w.progress() - 2.0 / 3.0).abs() < 1e-9);
        assert!(!w.is_finished());
        w.complete_current();
        assert!(w.is_finished());
        assert!(!w.next()); // letzter Schritt
        assert!(!w.go_to(3)); // gibt es nicht
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
