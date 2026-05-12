

pub fn code_find_symbols(
    code_query: &::code_graph::query::QueryEngine,
    query: &str,
    kind: Option<&str>,
    pattern: Option<&str>,
    limit: usize,
) -> Vec<::code_graph::types::Symbol> {
    let limit = limit.clamp(1, 100);
    let broad_limit = if query.trim().is_empty() {
        limit
    } else {
        10_000
    };

    let mut symbols = if let Some(pattern) = pattern.filter(|pattern| !pattern.trim().is_empty()) {
        if is_supported_code_pattern(pattern) {
            code_query
                .search_by_pattern(pattern, broad_limit)
                .unwrap_or_default()
        } else {
            search_code_symbols_with_fallback(code_query, pattern, broad_limit)
        }
    } else if let Some(kind) = kind.filter(|kind| !kind.trim().is_empty()) {
        symbols_for_kind(code_query, kind, broad_limit)
            .unwrap_or_else(|| search_code_symbols_with_fallback(code_query, query, broad_limit))
    } else {
        search_code_symbols_with_fallback(code_query, query, broad_limit)
    };

    filter_symbols_by_query(&mut symbols, query);
    symbols.truncate(limit);
    symbols
}

pub fn symbols_for_kind(
    code_query: &::code_graph::query::QueryEngine,
    kind: &str,
    limit: usize,
) -> Option<Vec<::code_graph::types::Symbol>> {
    let symbols = match kind.to_ascii_lowercase().as_str() {
        "function" | "fn" => code_query.functions(limit).unwrap_or_default(),
        "struct" => code_query.structs(limit).unwrap_or_default(),
        "class" => code_query.classes(limit).unwrap_or_default(),
        "enum" => code_query.enums(limit).unwrap_or_default(),
        _ => return None,
    };

    Some(symbols)
}

pub fn is_supported_code_pattern(pattern: &str) -> bool {
    matches!(
        pattern,
        "function_call"
            | "function_definition"
            | "struct_definition"
            | "struct"
            | "class_definition"
            | "class"
            | "enum_definition"
            | "enum"
            | "module_definition"
            | "module"
            | "import"
            | "use_statement"
    )
}

pub fn best_symbol_query_token(query: &str) -> Option<&str> {
    query
        .split(|ch: char| !(ch == '_' || ch.is_ascii_alphanumeric()))
        .filter(|token| !token.is_empty())
        .filter(|token| {
            !matches!(
                token.to_ascii_lowercase().as_str(),
                "fn" | "function" | "struct" | "class" | "enum" | "async" | "pub"
            )
        })
        .max_by_key(|token| token.len())
}

pub fn filter_symbols_by_query(symbols: &mut Vec<::code_graph::types::Symbol>, query: &str) {
    let query = query.trim().to_ascii_lowercase();
    if query.is_empty() {
        return;
    }

    symbols.retain(|symbol| {
        symbol.name.to_ascii_lowercase().contains(&query)
            || symbol
                .signature
                .as_deref()
                .unwrap_or_default()
                .to_ascii_lowercase()
                .contains(&query)
            || symbol.file_path.to_ascii_lowercase().contains(&query)
    });
}

pub fn search_code_symbols_with_fallback(
    code_query: &::code_graph::query::QueryEngine,
    query: &str,
    limit: usize,
) -> Vec<::code_graph::types::Symbol> {
    let query = query.trim();
    let mut symbols = code_query
        .search(query, limit)
        .map(|result| result.symbols)
        .unwrap_or_default();

    if symbols.is_empty() {
        if let Some(token) = best_symbol_query_token(query) {
            if token != query {
                symbols = code_query
                    .search(token, limit)
                    .map(|result| result.symbols)
                    .unwrap_or_default();
            }
        }
    }

    symbols
}
