use std::borrow::Cow;

/// An ascii table
#[derive(Debug)]
pub struct AsciiTable<'a> {
    data: Vec<Vec<Cow<'a, str>>>,

    max_cell_widths: Vec<usize>,

    padding: usize,
}

impl<'a> AsciiTable<'a> {
    /// Make a new table
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            data: vec![vec![Cow::Borrowed(""); width]; height],

            max_cell_widths: vec![0; width],

            padding: 0,
        }
    }

    /// Set the amount of spaces applied to each element
    pub fn set_padding(&mut self, padding: usize) {
        self.padding = padding;
    }

    /// Set the value of the given cell.
    ///
    /// Indexing starts at 0. It starts at the top left corner and ends at the bottom right.
    pub fn set_cell(&mut self, x: usize, y: usize, data: impl Into<Cow<'a, str>>) {
        let data = data.into();

        self.max_cell_widths[x] = std::cmp::max(self.max_cell_widths[x], data.len());
        self.data[y][x] = data;
    }

    fn fmt_row_border(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "+")?;
        for max_cell_width in self.max_cell_widths.iter() {
            for _ in 0..(*max_cell_width + (2 * self.padding)) {
                write!(f, "-")?;
            }
            write!(f, "+")?;
        }
        writeln!(f)?;

        Ok(())
    }
}

impl std::fmt::Display for AsciiTable<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for row in self.data.iter() {
            self.fmt_row_border(f)?;

            for (cell, max_cell_width) in row.iter().zip(self.max_cell_widths.iter()) {
                let mut padding = self.padding * 2;
                let cell_len = cell.len();
                if cell_len < *max_cell_width {
                    padding += max_cell_width - cell_len;
                }

                write!(f, "|")?;

                for _ in 0..padding / 2 {
                    write!(f, " ")?;
                }

                write!(f, "{}", cell)?;

                for _ in 0..((padding / 2) + padding % 2) {
                    write!(f, " ")?;
                }
            }
            writeln!(f, "|")?;
        }
        self.fmt_row_border(f)?;

        Ok(())
    }
}
