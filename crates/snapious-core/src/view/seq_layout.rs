//! Zeilenlayout der Sequenzansicht (reine Rechenlogik, ohne Oberfläche).
use crate::error::{Error, Result};

/// Aufteilung einer Sequenz in Zeilen fester Länge.
#[derive(Clone, Debug, PartialEq)]
pub struct SequenceLayout {
    length: usize,
    bases_per_line: usize,
}

impl SequenceLayout {
    pub fn new(length: usize, bases_per_line: usize) -> Result<Self> {
        if bases_per_line == 0 {
            return Err(Error::InvalidInput(
                "bases_per_line darf nicht 0 sein".to_string(),
            ));
        }
        Ok(Self {
            length,
            bases_per_line,
        })
    }

    pub fn line_count(&self) -> usize {
        if self.length == 0 {
            return 0;
        }
        self.length.div_ceil(self.bases_per_line)
    }

    pub fn line_of(&self, position: usize) -> usize {
        position / self.bases_per_line
    }

    pub fn column_of(&self, position: usize) -> usize {
        position % self.bases_per_line
    }

    pub fn line_start(&self, line: usize) -> usize {
        line * self.bases_per_line
    }

    pub fn line_end(&self, line: usize) -> usize {
        std::cmp::min(self.line_start(line) + self.bases_per_line, self.length)
    }

    pub fn first_visible_line(scroll_y: f64, line_height: f64) -> usize {
        if scroll_y <= 0.0 || line_height <= 0.0 {
            return 0;
        }
        (scroll_y / line_height).floor() as usize
    }

    pub fn last_visible_line(&self, scroll_y: f64, view_height: f64, line_height: f64) -> usize {
        if self.line_count() == 0 || line_height <= 0.0 {
            return 0;
        }
        let max_line = self.line_count() - 1;
        std::cmp::min(
            ((scroll_y + view_height) / line_height).floor() as usize,
            max_line,
        )
    }
}

pub fn bases_per_line_for(
    view_width: f64,
    char_width: f64,
    margin_width: f64,
    group_size: usize,
) -> usize {
    let gs = if group_size == 0 { 1 } else { group_size };
    let available = (view_width - margin_width).max(0.0);
    let group_width = (gs + 1) as f64 * char_width;
    if group_width == 0.0 {
        return gs;
    }
    let groups = (available / group_width).floor() as usize;
    (groups.max(1)) * gs
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zeilen_bei_laenge_125_und_60_pro_zeile() {
        let l = SequenceLayout::new(125, 60).unwrap();
        assert_eq!(l.line_count(), 3);
        assert_eq!(l.line_of(59), 0);
        assert_eq!(l.line_of(60), 1);
        assert_eq!(l.column_of(61), 1);
        assert_eq!(l.line_start(2), 120);
        assert_eq!(l.line_end(2), 125);
    }

    #[test]
    fn leere_sequenz_hat_keine_zeilen() {
        assert_eq!(SequenceLayout::new(0, 60).unwrap().line_count(), 0);
    }

    #[test]
    fn null_basen_pro_zeile_ist_fehler() {
        assert!(SequenceLayout::new(10, 0).is_err());
    }

    #[test]
    fn sichtbare_zeilen_bei_scrollposition_250() {
        let l = SequenceLayout::new(6000, 60).unwrap(); // genau 100 Zeilen
        assert_eq!(SequenceLayout::first_visible_line(250.0, 20.0), 12);
        assert_eq!(l.last_visible_line(250.0, 100.0, 20.0), 17);
    }

    #[test]
    fn letzte_zeile_wird_auf_vorhandene_zeilen_begrenzt() {
        let l = SequenceLayout::new(125, 60).unwrap(); // 3 Zeilen
        assert_eq!(l.last_visible_line(0.0, 10_000.0, 20.0), 2);
    }

    #[test]
    fn basen_pro_zeile_aus_fensterbreite() {
        assert_eq!(bases_per_line_for(800.0, 10.0, 80.0, 10), 60);
        assert_eq!(bases_per_line_for(50.0, 10.0, 0.0, 10), 10);
    }
}
