document.addEventListener('DOMContentLoaded', () => {
    // Initialize Mermaid with a dark theme that matches the Cyberpunk aesthetic
    if (typeof mermaid !== 'undefined') {
        mermaid.initialize({
            startOnLoad: true,
            theme: 'dark',
            themeVariables: {
                primaryColor: '#00FF41',
                primaryTextColor: '#f4f4f5',
                primaryBorderColor: '#3f3f46',
                lineColor: '#22c55e',
                secondaryColor: '#18181b',
                tertiaryColor: '#27272a'
            }
        });
    }

    // Helper: Format dates found in the document
    const dateElements = document.querySelectorAll('.date-format');
    dateElements.forEach(el => {
        const date = new Date(el.textContent);
        if (!isNaN(date)) {
            el.textContent = date.toLocaleDateString(undefined, {
                year: 'numeric',
                month: 'long',
                day: 'numeric'
            });
        }
    });

    console.log('Xavier DevLog Theme Initialized');
});
