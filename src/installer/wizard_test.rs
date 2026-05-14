#[cfg(test)]
mod tests {
    use crate::installer::wizard::render_input_field;
    use ratatui::{backend::TestBackend, Terminal, layout::Rect};

    #[test]
    fn test_render_input_field_masked_multibyte() {
        let backend = TestBackend::new(40, 3);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal.draw(|f| {
            render_input_field(
                f,
                Rect::new(0, 0, 40, 1),
                "Label",
                "value",
                false,
                0,
                true,
            );
        }).unwrap();

        let buf = terminal.backend().buffer();
        // "Label: " is 7 chars. "value" is 5 chars.
        // Masked should show 5 bullets.
        // Total: "Label: •••••"
        let mut rendered = String::new();
        for i in 0..12 {
            rendered.push(buf[(i, 0)].symbol().chars().next().unwrap());
        }
        assert_eq!(rendered, "Label: •••••");

    }

    #[test]
    fn test_render_input_field_unmasked_multibyte() {
        let backend = TestBackend::new(40, 3);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal.draw(|f| {
            render_input_field(
                f,
                Rect::new(0, 0, 40, 1),
                "Label",
                "🦀Rust",
                false,
                0,
                false,
            );
        }).unwrap();

        let buf = terminal.backend().buffer();

        // Check label
        let mut label = String::new();
        for i in 0..7 {
            label.push_str(buf[(i, 0)].symbol());
        }
        assert_eq!(label, "Label: ");


        // Check emoji
        assert_eq!(buf[(7, 0)].symbol(), "🦀");

        // The emoji 🦀 is width 2. Index 8 is the filler.
        let rust_start = 9;

        let mut rust = String::new();
        for i in rust_start..rust_start+4 {
            rust.push_str(buf[(i, 0)].symbol());
        }
        assert_eq!(rust, "Rust");
    }

    #[test]
    fn test_render_input_field_cursor_multibyte() {
        let backend = TestBackend::new(40, 3);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal.draw(|f| {
            render_input_field(
                f,
                Rect::new(0, 0, 40, 1),
                "Input",
                "A🦀C",
                true,
                1, // cursor at 🦀
                false,
            );
        }).unwrap();

        let buf = terminal.backend().buffer();
        // "Input: " (7) + "A" (1) + "🦀" (1) + "C" (1) = 10
        // Cursor at pos 1 of "A🦀C" means it's on "🦀"
        assert_eq!(buf[(8, 0)].symbol(), "🦀");
        // Check background color of cursor cell (ACCENT is Color::Cyan)
        // ACCENT is Color::Cyan
        use ratatui::style::Color;
        assert_eq!(buf[(8, 0)].bg, Color::Cyan);
    }
}
