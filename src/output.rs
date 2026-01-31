pub struct Styler {
    color_enabled: bool,
}

impl Styler {
    pub fn new(color_enabled: bool) -> Self {
        Self { color_enabled }
    }

    fn wrap(&self, code: &str, s: &str) -> String {
        if !self.color_enabled {
            return s.to_string();
        }
        format!("{}{}\u{001b}[0m", code, s)
    }

    pub fn green(&self, s: &str) -> String {
        self.wrap("\u{001b}[32m", s)
    }

    pub fn gray(&self, s: &str) -> String {
        self.wrap("\u{001b}[90m", s)
    }
}

fn pad_right(s: &str, width: usize) -> String {
    if s.len() >= width {
        s.to_string()
    } else {
        let mut out = String::with_capacity(width);
        out.push_str(s);
        out.push_str(&" ".repeat(width - s.len()));
        out
    }
}

pub fn render_simple_table(headers: &[&str], rows: &[Vec<String>]) -> String {
    let mut widths: Vec<usize> = headers.iter().map(|h| h.len()).collect();

    for row in rows.iter() {
        for (i, cell) in row.iter().enumerate() {
            if i >= widths.len() {
                widths.push(cell.len());
            } else {
                widths[i] = widths[i].max(cell.len());
            }
        }
    }

    let header_line = headers
        .iter()
        .enumerate()
        .map(|(i, h)| pad_right(h, widths[i]))
        .collect::<Vec<String>>()
        .join("  ");

    let mut body_lines: Vec<String> = Vec::new();
    for row in rows.iter() {
        let line = row
            .iter()
            .enumerate()
            .map(|(i, cell)| pad_right(cell, widths[i]))
            .collect::<Vec<String>>()
            .join("  ");
        body_lines.push(line);
    }

    if body_lines.is_empty() {
        header_line
    } else {
        format!("{}\n{}", header_line, body_lines.join("\n"))
    }
}
