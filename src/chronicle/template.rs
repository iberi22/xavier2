use chrono::Local;

pub struct ChronicleTemplate;

impl ChronicleTemplate {
    pub fn render(content: &str) -> String {
        let date = Local::now().format("%Y-%m-%d");
        format!(
            "---\ntitle: \"Daily Chronicle - {}\"\ndate: {}\ntags: [\"automated\", \"devlog\", \"xavier2\"]\n---\n\n{}",
            date, date, content
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_template() {
        let content = "# Hello World\nThis is a test.";
        let rendered = ChronicleTemplate::render(content);
        assert!(rendered.contains("Daily Chronicle"));
        assert!(rendered.contains("# Hello World"));
        assert!(rendered.contains("---"));
    }
}
