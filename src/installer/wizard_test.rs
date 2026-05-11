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
                "Token",
                "🔑secret",
                false,
                0,
                true,
            );
        }).unwrap();

        let buf = terminal.backend().buffer();
        // "Token: " is 7 chars. "🔑secret" is 7 chars (1 emoji + 6 letters).
        // Masked should show 7 bullets.
        // Total: "Token: •••••••"
        let mut rendered = String::new();
        for i in 0..14 {
            rendered.push(buf[(i, 0)].symbol().chars().next().unwrap());
        }
        assert_eq!(rendered, "Token: •••••••");
    }

    #[test]
    fn test_render_input_field_unmasked_multibyte() {
        let backend = TestBackend::new(40, 3);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal.draw(|f| {
            render_input_field(
                f,
                Rect::new(0, 0, 40, 1),
                "Name",
                "🦀Rust",
                false,
                0,
                false,
            );
        }).unwrap();

        let buf = terminal.backend().buffer();

        // Check label
        let mut label = String::new();
        for i in 0..6 {
            label.push_str(buf[(i, 0)].symbol());
        }
        assert_eq!(label, "Name: ");

        // Check emoji
        assert_eq!(buf[(6, 0)].symbol(), "🦀");

        // The emoji 🦀 is width 2. Ratatui renders it in one cell and
        // usually leaves the next cell empty or with a placeholder.
        // In this environment, it seems cell 7 is a space.
        let rust_start = if buf[(7, 0)].symbol() == " " || buf[(7, 0)].symbol() == "" {
            8
        } else {
            7
        };

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
        // Check background color of cursor cell (ACCENT is Cyan)
        // ACCENT is Color::Cyan
        use ratatui::style::Color;
        assert_eq!(buf[(8, 0)].bg, Color::Cyan);
    }
}
