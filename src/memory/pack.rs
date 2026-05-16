use crate::retrieval::gating::LayeredSearchResult;
use std::fmt::Write;

/// Generates a Context Pack (.xcp) in XML format from layered search results.
pub fn generate_xcp(result: LayeredSearchResult, max_level: usize) -> String {
    let mut xml = String::new();
    let topic_escaped = escape_xml(&result.topic);

    writeln!(
        xml,
        "<xavier_context_pack topic=\"{}\" generated=\"{}\">",
        topic_escaped, result.timestamp
    ).unwrap();

    // Level 0: Working Memory
    {
        writeln!(xml, "  <level_0_working_memory>").unwrap();
        for r in result.level_0_working {
            writeln!(xml, "    <item score=\"{:.3}\" path=\"{}\">", r.score, escape_xml(&r.path)).unwrap();
            writeln!(xml, "      {}", escape_xml(&r.content)).unwrap();
            writeln!(xml, "    </item>").unwrap();
        }
        writeln!(xml, "  </level_0_working_memory>").unwrap();
    }

    // Level 1: Entity Graph
    if max_level >= 1 {
        writeln!(xml, "  <level_1_entity_graph>").unwrap();
        for r in result.level_1_entity_graph {
            writeln!(xml, "    <node score=\"{:.3}\" path=\"{}\">", r.score, escape_xml(&r.path)).unwrap();
            writeln!(xml, "      {}", escape_xml(&r.content)).unwrap();
            writeln!(xml, "    </node>").unwrap();
        }
        writeln!(xml, "  </level_1_entity_graph>").unwrap();
    }

    // Level 2: Semantic (Rules/Definitions)
    if max_level >= 2 {
        writeln!(xml, "  <level_2_semantic>").unwrap();
        for r in result.level_2_semantic {
            writeln!(xml, "    <definition score=\"{:.3}\" path=\"{}\">", r.score, escape_xml(&r.path)).unwrap();
            writeln!(xml, "      {}", escape_xml(&r.content)).unwrap();
            writeln!(xml, "    </definition>").unwrap();
        }
        writeln!(xml, "  </level_2_semantic>").unwrap();
    }

    // Level 3: Episodic (History/Logs)
    if max_level >= 3 {
        writeln!(xml, "  <level_3_episodic>").unwrap();
        for r in result.level_3_episodic {
            writeln!(xml, "    <event score=\"{:.3}\" path=\"{}\">", r.score, escape_xml(&r.path)).unwrap();
            writeln!(xml, "      {}", escape_xml(&r.content)).unwrap();
            writeln!(xml, "    </event>").unwrap();
        }
        writeln!(xml, "  </level_3_episodic>").unwrap();
    }

    writeln!(xml, "</xavier_context_pack>").unwrap();
    xml
}

fn escape_xml(s: &str) -> String {
    let mut escaped = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '&' => escaped.push_str("&amp;"),
            '\'' => escaped.push_str("&apos;"),
            '"' => escaped.push_str("&quot;"),
            _ => escaped.push(c),
        }
    }
    escaped
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::search::rrf::ScoredResult;

    #[test]
    fn test_xcp_generation() {
        let result = LayeredSearchResult {
            topic: "test topic".to_string(),
            timestamp: "2023-10-27T10:00:00Z".to_string(),
            level_0_working: vec![ScoredResult {
                id: "id1".to_string(),
                content: "working content".to_string(),
                score: 0.9,
                source: "working".to_string(),
                path: "path/1".to_string(),
                updated_at: None,
            }],
            level_1_entity_graph: vec![],
            level_2_semantic: vec![],
            level_3_episodic: vec![],
        };

        let xcp = generate_xcp(result, 3);
        assert!(xcp.contains("<xavier_context_pack topic=\"test topic\""));
        assert!(xcp.contains("<level_0_working_memory>"));
        assert!(xcp.contains("working content"));
        assert!(xcp.contains("</xavier_context_pack>"));
    }

    #[test]
    fn test_max_level_filtering() {
        let result = LayeredSearchResult {
            topic: "test topic".to_string(),
            timestamp: "2023-10-27T10:00:00Z".to_string(),
            level_0_working: vec![],
            level_1_entity_graph: vec![ScoredResult {
                id: "id2".to_string(),
                content: "entity content".to_string(),
                score: 0.8,
                source: "semantic".to_string(),
                path: "path/2".to_string(),
                updated_at: None,
            }],
            level_2_semantic: vec![],
            level_3_episodic: vec![],
        };

        let xcp_0 = generate_xcp(result.clone(), 0);
        assert!(!xcp_0.contains("<level_1_entity_graph>"));

        let xcp_1 = generate_xcp(result.clone(), 1);
        assert!(xcp_1.contains("<level_1_entity_graph>"));
    }
}
