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
                "Value",
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


        // Check label and start of value
        // The value might be "(empty)" if it was empty, but here it's "Value"
        let mut rendered_value = String::new();
        for i in 7..12 {
            rendered_value.push_str(buf[(i, 0)].symbol());
        }
        assert!(rendered_value.contains("Value"));
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
        // Check background color of cursor cell (ACCENT is Cyan)
        // ACCENT is Color::Cyan
        use ratatui::style::Color;
        assert_eq!(buf[(8, 0)].bg, Color::Cyan);
    }
}
