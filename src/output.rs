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

/// Calculate display width of a string, accounting for Unicode/emoji.
/// Uses a simple heuristic: most CJK/emoji chars are width 2, others are 1.
/// This is deterministic and doesn't probe terminal width.
pub fn display_width(s: &str) -> usize {
    s.chars()
        .map(|c| {
            // Wide characters: CJK, emoji, fullwidth forms
            // Simplified ranges covering common wide characters
            if is_wide_char(c) {
                2
            } else {
                1
            }
        })
        .sum()
}

/// Check if a character is "wide" (typically renders as 2 columns).
fn is_wide_char(c: char) -> bool {
    let cp = c as u32;
    // CJK Unified Ideographs and extensions
    (0x4E00..=0x9FFF).contains(&cp) ||
    (0x3400..=0x4DBF).contains(&cp) ||
    (0x20000..=0x2A6DF).contains(&cp) ||
    // CJK Compatibility Ideographs
    (0xF900..=0xFAFF).contains(&cp) ||
    // Fullwidth forms
    (0xFF00..=0xFFEF).contains(&cp) ||
    // Hangul Syllables
    (0xAC00..=0xD7AF).contains(&cp) ||
    // Common emoji ranges
    (0x1F300..=0x1F9FF).contains(&cp) ||  // Misc Symbols, Emoticons, etc.
    (0x1F600..=0x1F64F).contains(&cp) ||  // Emoticons
    (0x1F680..=0x1F6FF).contains(&cp) ||  // Transport/Map
    (0x2600..=0x26FF).contains(&cp) ||    // Misc symbols
    (0x2700..=0x27BF).contains(&cp) ||    // Dingbats
    // Block elements (used in progress bars)
    (0x2580..=0x259F).contains(&cp)
}

fn pad_right(s: &str, width: usize) -> String {
    let dw = display_width(s);
    if dw >= width {
        s.to_string()
    } else {
        let mut out = String::with_capacity(s.len() + (width - dw));
        out.push_str(s);
        out.push_str(&" ".repeat(width - dw));
        out
    }
}

pub fn render_simple_table(headers: &[&str], rows: &[Vec<String>]) -> String {
    let mut widths: Vec<usize> = headers.iter().map(|h| display_width(h)).collect();

    for row in rows.iter() {
        for (i, cell) in row.iter().enumerate() {
            let cell_width = display_width(cell);
            if i >= widths.len() {
                widths.push(cell_width);
            } else {
                widths[i] = widths[i].max(cell_width);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_display_width_ascii() {
        assert_eq!(display_width("hello"), 5);
        assert_eq!(display_width(""), 0);
        assert_eq!(display_width("a"), 1);
    }

    #[test]
    fn test_display_width_cjk() {
        // CJK characters are width 2
        assert_eq!(display_width("한"), 2);      // Hangul
        assert_eq!(display_width("中"), 2);      // Chinese
        assert_eq!(display_width("日本"), 4);    // Japanese (2 chars)
        assert_eq!(display_width("Hello中文"), 9); // 5 ASCII + 2 CJK chars
    }

    #[test]
    fn test_display_width_emoji() {
        // Emoji are width 2
        assert_eq!(display_width("😀"), 2);
        assert_eq!(display_width("🚀"), 2);
        assert_eq!(display_width("Test🎉"), 6);  // 4 ASCII + 1 emoji
    }

    #[test]
    fn test_display_width_block_elements() {
        // Block elements (used in progress bars) are width 2
        assert_eq!(display_width("█"), 2);
        assert_eq!(display_width("░"), 2);
        assert_eq!(display_width("█████"), 10);  // 5 blocks = 10 width
    }

    #[test]
    fn test_pad_right_ascii() {
        assert_eq!(pad_right("hi", 5), "hi   ");
        assert_eq!(pad_right("hello", 5), "hello");
        assert_eq!(pad_right("toolong", 5), "toolong");
    }

    #[test]
    fn test_pad_right_unicode() {
        // CJK char is width 2, so "한" needs 3 spaces to reach width 5
        assert_eq!(pad_right("한", 5), "한   ");
        // "中文" is width 4, needs 1 space to reach width 5
        assert_eq!(pad_right("中文", 5), "中文 ");
    }

    #[test]
    fn test_render_simple_table_unicode_alignment() {
        let headers = &["name", "value"];
        let rows = vec![
            vec!["한글".to_string(), "100".to_string()],
            vec!["ASCII".to_string(), "200".to_string()],
        ];
        let table = render_simple_table(headers, &rows);
        let lines: Vec<&str> = table.lines().collect();

        // Both data rows should have same length for alignment
        assert_eq!(lines.len(), 3);
        // The display width of each line should match
        assert_eq!(display_width(lines[1]), display_width(lines[2]));
    }
}
