use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};
use tokio::sync::RwLock;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum EntityType {
    Person,
    Organization,
    Location,
    Product,
    Concept,
    Unknown,
}

impl EntityType {
    fn as_str(self) -> &'static str {
        match self {
            Self::Person => "person",
            Self::Organization => "organization",
            Self::Location => "location",
            Self::Product => "product",
            Self::Concept => "concept",
            Self::Unknown => "unknown",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedEntity {
    pub name: String,
    pub entity_type: EntityType,
    pub span: (usize, usize),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityRecord {
    pub id: String,
    pub name: String,
    pub normalized_name: String,
    pub entity_type: EntityType,
    #[serde(default)]
    pub aliases: Vec<String>,
    pub description: Option<String>,
    pub occurrence_count: usize,
    pub memory_count: usize,
    pub first_seen: DateTime<Utc>,
    pub last_seen: DateTime<Utc>,
    #[serde(default)]
    pub merged_from: Vec<String>,
    /// Trust score [0.0, 1.0] based on confirmation count (default 0.5)
    #[serde(default)]
    pub trust_score: f32,
    /// Trust rank for ordering (higher = more trusted)
    #[serde(default)]
    pub trust_rank: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityRelationRecord {
    pub id: String,
    pub source: String,
    pub target: String,
    pub relation_type: String,
    pub weight: f32,
    pub co_occurrence_score: f32,
    pub support_count: usize,
    #[serde(default)]
    pub provenance: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GraphDirection {
    Outgoing,
    Incoming,
    Both,
}

impl Default for GraphDirection {
    fn default() -> Self {
        Self::Outgoing
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraversalStep {
    pub from: String,
    pub to: String,
    pub relation_type: String,
    pub depth: usize,
    pub weight: f32,
    #[serde(default)]
    pub path: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityNeighbors {
    pub entity: EntityRecord,
    pub incoming: Vec<EntityRelationRecord>,
    pub outgoing: Vec<EntityRelationRecord>,
    pub traversal: Vec<TraversalStep>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphRelationsView {
    pub entity_id: Option<String>,
    pub direction: GraphDirection,
    pub max_depth: usize,
    pub total_relations: usize,
    pub relations: Vec<EntityRelationRecord>,
    #[serde(default)]
    pub traversal: Vec<TraversalStep>,
}

#[derive(Debug, Clone, Default)]
struct GraphData {
    entities: HashMap<String, EntityRecord>,
    entity_lookup: HashMap<String, String>,
    relations: HashMap<String, EntityRelationRecord>,
    relation_lookup: HashMap<String, String>,
    outgoing: HashMap<String, HashSet<String>>,
    incoming: HashMap<String, HashSet<String>>,
    memory_entities: HashMap<String, HashSet<String>>,
}

#[derive(Debug, Default)]
pub struct EntityGraph {
    inner: RwLock<GraphData>,
}

pub type SharedEntityGraph = std::sync::Arc<EntityGraph>;

static CANDIDATE_ENTITY_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\b(?:[A-Z]{2,}(?:[A-Z0-9_-]*[A-Z0-9])?|[A-Z][a-z0-9]+(?:\s+[A-Z][a-z0-9]+)*|[A-Za-z]+[0-9]+[A-Za-z0-9_-]*)\b")
        .unwrap()
});
static EMAIL_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"[\w.+-]+@[\w-]+\.[\w.-]+").unwrap());
static URL_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"https?://[^\s]+").unwrap());

static RELATION_PATTERNS: &[(&str, &str, f32)] = &[
    (
        r"(?i)\b(?P<source>[A-Z][\w-]*(?:\s+[A-Z][\w-]*)*)\s+works?\s+at\s+(?P<target>[A-Z][\w-]*(?:\s+[A-Z][\w-]*)*)",
        "works_at",
        0.95,
    ),
    (
        r"(?i)\b(?P<source>[A-Z][\w-]*(?:\s+[A-Z][\w-]*)*)\s+knows?\s+(?P<target>[A-Z][\w-]*(?:\s+[A-Z][\w-]*)*)",
        "knows",
        0.9,
    ),
    (
        r"(?i)\b(?P<source>[A-Z][\w-]*(?:\s+[A-Z][\w-]*)*)\s+uses?\s+(?P<target>[A-Z][\w-]*(?:\s+[A-Z][\w-]*)*)",
        "uses",
        0.85,
    ),
    (
        r"(?i)\b(?P<source>[A-Z][\w-]*(?:\s+[A-Z][\w-]*)*)\s+is\s+a[n]?\s+(?P<target>[A-Z][\w-]*(?:\s+[A-Z][\w-]*)*)",
        "is_a",
        0.8,
    ),
    (
        r"(?i)\b(?P<source>[A-Z][\w-]*(?:\s+[A-Z][\w-]*)*)\s+part\s+of\s+(?P<target>[A-Z][\w-]*(?:\s+[A-Z][\w-]*)*)",
        "part_of",
        0.9,
    ),
    (
        r"(?i)\b(?P<source>[A-Z][\w-]*(?:\s+[A-Z][\w-]*)*)\s+located\s+in\s+(?P<target>[A-Z][\w-]*(?:\s+[A-Z][\w-]*)*)",
        "located_in",
        0.9,
    ),
    (
        r"(?i)\b(?P<source>[A-Z][\w-]*(?:\s+[A-Z][\w-]*)*)\s+related\s+to\s+(?P<target>[A-Z][\w-]*(?:\s+[A-Z][\w-]*)*)",
        "related_to",
        0.7,
    ),
];

static COMMON_WORDS: &[&str] = &[
    "the", "this", "that", "these", "those", "and", "or", "but", "for", "with", "from", "into",
    "onto", "your", "our", "their", "his", "her", "its", "in", "on", "at", "by", "to", "of",
];

impl EntityGraph {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn upsert_memory(
        &self,
        memory_id: &str,
        content: &str,
        metadata: Option<&serde_json::Value>,
    ) -> Result<GraphRelationsView> {
        let extracted = Self::extract_entities(content);
        self.index_memory(memory_id, content, metadata, extracted).await
    }

    pub async fn remove_memory(&self, memory_id: &str) -> Result<()> {
        let mut data = self.inner.write().await;
        if let Some(entities) = data.memory_entities.remove(memory_id) {
            for entity_id in entities {
                if let Some(entity) = data.entities.get_mut(&entity_id) {
                    entity.memory_count = entity.memory_count.saturating_sub(1);
                    entity.last_seen = Utc::now();
                }
            }
        }

        data.relations.retain(|_, relation| {
            !relation.provenance.iter().any(|provenance| provenance == memory_id)
        });
        data.rebuild_indexes();
        Ok(())
    }

    pub async fn entity(&self, entity_id_or_name: &str) -> Option<EntityRecord> {
        let data = self.inner.read().await;
        data.resolve_entity_id(entity_id_or_name)
            .and_then(|id| data.entities.get(&id).cloned())
    }

    pub async fn all_entities(&self) -> Vec<EntityRecord> {
        self.inner.read().await.entities.values().cloned().collect()
    }

    pub async fn all_relations(&self) -> Vec<EntityRelationRecord> {
        self.inner.read().await.relations.values().cloned().collect()
    }

    pub async fn relations_for_entity(
        &self,
        entity_id_or_name: &str,
        max_depth: usize,
        relation_types: Option<&[String]>,
        direction: GraphDirection,
    ) -> Result<GraphRelationsView> {
        let data = self.inner.read().await;
        let entity_id = data
            .resolve_entity_id(entity_id_or_name)
            .ok_or_else(|| anyhow!("entity not found: {entity_id_or_name}"))?;
        let traversal =
            Self::traverse_locked(&data, &entity_id, max_depth, relation_types, direction);
        let relations = Self::relations_from_locked(&data, &entity_id, relation_types, direction);

        Ok(GraphRelationsView {
            entity_id: Some(entity_id),
            direction,
            max_depth,
            total_relations: relations.len(),
            relations,
            traversal,
        })
    }

    pub async fn entity_neighbors(
        &self,
        entity_id_or_name: &str,
        max_depth: usize,
        relation_types: Option<&[String]>,
        direction: GraphDirection,
    ) -> Result<EntityNeighbors> {
        let data = self.inner.read().await;
        let entity_id = data
            .resolve_entity_id(entity_id_or_name)
            .ok_or_else(|| anyhow!("entity not found: {entity_id_or_name}"))?;
        let entity = data
            .entities
            .get(&entity_id)
            .cloned()
            .ok_or_else(|| anyhow!("entity not found: {entity_id}"))?;
        let traversal =
            Self::traverse_locked(&data, &entity_id, max_depth, relation_types, direction);
        let incoming = Self::relations_from_ids_locked(&data, &entity_id, relation_types, true);
        let outgoing = Self::relations_from_ids_locked(&data, &entity_id, relation_types, false);

        Ok(EntityNeighbors {
            entity,
            incoming,
            outgoing,
            traversal,
        })
    }

    pub async fn merge_entities(&self, primary_id: &str, secondary_id: &str) -> Result<EntityRecord> {
        let mut data = self.inner.write().await;
        let primary_id = data
            .resolve_entity_id(primary_id)
            .ok_or_else(|| anyhow!("primary entity not found: {primary_id}"))?;
        let secondary_id = data
            .resolve_entity_id(secondary_id)
            .ok_or_else(|| anyhow!("secondary entity not found: {secondary_id}"))?;
        if primary_id == secondary_id {
            return data
                .entities
                .get(&primary_id)
                .cloned()
                .ok_or_else(|| anyhow!("entity not found: {primary_id}"));
        }

        let Some(mut secondary) = data.entities.remove(&secondary_id) else {
            return Err(anyhow!("secondary entity not found: {secondary_id}"));
        };
        let merged = {
            let Some(primary) = data.entities.get_mut(&primary_id) else {
                return Err(anyhow!("primary entity not found: {primary_id}"));
            };

            primary.aliases.push(secondary.name.clone());
            primary.aliases.extend(secondary.aliases.drain(..));
            primary.aliases.sort();
            primary.aliases.dedup();
            primary.occurrence_count += secondary.occurrence_count;
            primary.memory_count += secondary.memory_count;
            primary.merged_from.push(secondary.id.clone());
            primary.merged_from.extend(secondary.merged_from.drain(..));
            primary.merged_from.sort();
            primary.merged_from.dedup();
            primary.last_seen = primary.last_seen.max(secondary.last_seen);
            if primary.description.is_none() {
                primary.description = secondary.description.take();
            }

            primary.clone()
        };

        for entity_id in data.relation_neighbors(&secondary_id) {
            if entity_id == primary_id {
                continue;
            }
            data.relink_relation_neighbor(&secondary_id, &primary_id, &entity_id);
        }
        data.remove_relations_for_entity(&secondary_id);
        data.rebuild_indexes();
        Ok(merged)
    }

    pub fn extract_entities(text: &str) -> Vec<ExtractedEntity> {
        let mut seen = HashSet::new();
        let mut entities = Vec::new();
        let explicit_relations = Self::extract_relation_candidates(text);
        let mut subject_names = HashSet::new();
        let mut object_names = HashSet::new();
        for relation in &explicit_relations {
            subject_names.insert(normalize_name(&relation.source));
            object_names.insert(normalize_name(&relation.target));
        }

        for mat in EMAIL_RE.find_iter(text) {
            let name = mat.as_str().trim().to_string();
            let key = format!("{}|{:?}", normalize_name(&name), EntityType::Concept);
            if seen.insert(key) {
                entities.push(ExtractedEntity {
                    name,
                    entity_type: EntityType::Concept,
                    span: (mat.start(), mat.end()),
                });
            }
        }

        for mat in URL_RE.find_iter(text) {
            let name = mat.as_str().trim().to_string();
            let key = format!("{}|{:?}", normalize_name(&name), EntityType::Product);
            if seen.insert(key) {
                entities.push(ExtractedEntity {
                    name,
                    entity_type: EntityType::Product,
                    span: (mat.start(), mat.end()),
                });
            }
        }

        for mat in CANDIDATE_ENTITY_RE.find_iter(text) {
            let name = mat
                .as_str()
                .trim()
                .trim_matches(|c: char| {
                    matches!(c, ',' | '.' | ';' | ':' | '"' | '\'' | '(' | ')' | '[' | ']')
                })
                .to_string();
            if is_common_word(&name) {
                continue;
            }
            let normalized = normalize_name(&name);
            let entity_type = Self::guess_entity_type(&name, &subject_names, &object_names);
            let key = format!("{}|{:?}", normalized, entity_type);
            if seen.insert(key) {
                entities.push(ExtractedEntity {
                    name,
                    entity_type,
                    span: (mat.start(), mat.end()),
                });
            }
        }

        entities
    }

    fn guess_entity_type(
        name: &str,
        subject_names: &HashSet<String>,
        object_names: &HashSet<String>,
    ) -> EntityType {
        let normalized = normalize_name(name);
        let lowered = normalized.to_ascii_lowercase();

        if subject_names.contains(&normalized) {
            return EntityType::Person;
        }
        if object_names.contains(&normalized) {
            if looks_like_location(&lowered) {
                return EntityType::Location;
            }
            if looks_like_organization(name) {
                return EntityType::Organization;
            }
        }
        if looks_like_location(&lowered) {
            return EntityType::Location;
        }
        if looks_like_organization(name) {
            return EntityType::Organization;
        }
        if looks_like_product(name) {
            return EntityType::Product;
        }
        if looks_like_person(name) {
            return EntityType::Person;
        }
        EntityType::Concept
    }

    pub fn extract_relation_candidates(text: &str) -> Vec<RawRelation> {
        let entities = Self::extract_entities_without_relations(text);
        let mut relations = Vec::new();
        for (pattern, relation_type, score) in RELATION_PATTERNS {
            let re = Regex::new(pattern).unwrap();
            for cap in re.captures_iter(text) {
                let Some(source) = cap.name("source").map(|m| m.as_str().trim()) else {
                    continue;
                };
                let Some(target) = cap.name("target").map(|m| m.as_str().trim()) else {
                    continue;
                };
                let source =
                    Self::best_match(source, &entities).unwrap_or_else(|| source.to_string());
                let target =
                    Self::best_match(target, &entities).unwrap_or_else(|| target.to_string());
                relations.push(RawRelation {
                    source,
                    target,
                    relation_type: relation_type.to_string(),
                    score: *score,
                });
            }
        }
        relations
    }

    fn extract_entities_without_relations(text: &str) -> Vec<String> {
        CANDIDATE_ENTITY_RE
            .find_iter(text)
            .map(|mat| {
                mat.as_str()
                    .trim()
                    .trim_matches(|c: char| {
                        matches!(c, ',' | '.' | ';' | ':' | '"' | '\'' | '(' | ')' | '[' | ']')
                    })
                    .to_string()
            })
            .filter(|name| !is_common_word(name))
            .collect()
    }

    fn best_match(candidate: &str, entities: &[String]) -> Option<String> {
        let normalized = normalize_name(candidate);
        entities
            .iter()
            .find(|entity| normalize_name(entity) == normalized)
            .cloned()
    }

    async fn index_memory(
        &self,
        memory_id: &str,
        content: &str,
        metadata: Option<&serde_json::Value>,
        extracted: Vec<ExtractedEntity>,
    ) -> Result<GraphRelationsView> {
        let now = Utc::now();
        let mut data = self.inner.write().await;
        let memory_key = memory_id.to_string();
        let mut entity_ids = Vec::new();
        let mut seen_entities = HashSet::new();

        if let Some(existing_entities) = data.memory_entities.remove(&memory_key) {
            for entity_id in existing_entities {
                if let Some(entity) = data.entities.get_mut(&entity_id) {
                    entity.memory_count = entity.memory_count.saturating_sub(1);
                }
            }
        }
        data.relations.retain(|_, relation| {
            !relation.provenance.iter().any(|provenance| provenance == memory_id)
        });

        for entity in extracted {
            let entity_id = data.upsert_entity(entity, &memory_key, metadata, now);
            if seen_entities.insert(entity_id.clone()) {
                entity_ids.push(entity_id);
            }
        }

        let mut created_relations = Vec::new();
        let co_occurrence_score = Self::co_occurrence_score(entity_ids.len());
        for i in 0..entity_ids.len() {
            for j in (i + 1)..entity_ids.len() {
                let source = entity_ids[i].clone();
                let target = entity_ids[j].clone();
                let relation = data.upsert_relation(
                    &source,
                    &target,
                    "co_occurs_with",
                    co_occurrence_score,
                    co_occurrence_score,
                    Some(memory_id),
                    now,
                );
                created_relations.push(relation.clone());
                created_relations.push(data.upsert_relation(
                    &target,
                    &source,
                    "co_occurs_with",
                    co_occurrence_score,
                    co_occurrence_score,
                    Some(memory_id),
                    now,
                ));
            }
        }

        for raw_relation in Self::extract_relation_candidates(content) {
            let Some(source_id) = data.resolve_entity_id(&raw_relation.source) else {
                continue;
            };
            let Some(target_id) = data.resolve_entity_id(&raw_relation.target) else {
                continue;
            };
            let relation = data.upsert_relation(
                &source_id,
                &target_id,
                &raw_relation.relation_type,
                raw_relation.score,
                0.0,
                Some(memory_id),
                now,
            );
            created_relations.push(relation);
        }

        if let Some(metadata) = metadata {
            if let Some(description) = metadata.get("description").and_then(|value| value.as_str()) {
                if let Some(first_id) = entity_ids.first() {
                    if let Some(entity) = data.entities.get_mut(first_id) {
                        if entity.description.is_none() {
                            entity.description = Some(description.to_string());
                        }
                    }
                }
            }
        }

        data.memory_entities
            .insert(memory_key.clone(), entity_ids.iter().cloned().collect());
        data.rebuild_indexes();

        let relations: Vec<_> = created_relations
            .into_iter()
            .filter(|relation| relation.provenance.iter().any(|item| item == memory_id))
            .collect();
        Ok(GraphRelationsView {
            entity_id: data
                .memory_entities
                .get(&memory_key)
                .and_then(|set| set.iter().next().cloned()),
            direction: GraphDirection::Both,
            max_depth: 1,
            total_relations: relations.len(),
            relations,
            traversal: Vec::new(),
        })
    }

    fn co_occurrence_score(entity_count: usize) -> f32 {
        match entity_count {
            0 | 1 => 0.0,
            2 => 0.55,
            3 => 0.65,
            4 => 0.75,
            _ => 0.85,
        }
    }

    fn traverse_locked(
        data: &GraphData,
        start_entity: &str,
        max_depth: usize,
        relation_types: Option<&[String]>,
        direction: GraphDirection,
    ) -> Vec<TraversalStep> {
        let mut visited = HashSet::new();
        let mut queue =
            VecDeque::from([(start_entity.to_string(), 0usize, vec![start_entity.to_string()])]);
        let mut steps = Vec::new();

        while let Some((entity_id, depth, path)) = queue.pop_front() {
            if depth >= max_depth || !visited.insert((entity_id.clone(), depth)) {
                continue;
            }

            for relation in Self::relations_from_locked(data, &entity_id, relation_types, direction) {
                let next = if relation.source == entity_id {
                    relation.target.clone()
                } else {
                    relation.source.clone()
                };
                let mut next_path = path.clone();
                next_path.push(next.clone());
                steps.push(TraversalStep {
                    from: relation.source.clone(),
                    to: relation.target.clone(),
                    relation_type: relation.relation_type.clone(),
                    depth,
                    weight: relation.weight,
                    path: next_path.clone(),
                });
                queue.push_back((next, depth + 1, next_path));
            }
        }

        steps
    }

    fn relations_from_locked(
        data: &GraphData,
        entity_id: &str,
        relation_types: Option<&[String]>,
        direction: GraphDirection,
    ) -> Vec<EntityRelationRecord> {
        match direction {
            GraphDirection::Outgoing => {
                Self::relations_from_ids_locked(data, entity_id, relation_types, false)
            }
            GraphDirection::Incoming => {
                Self::relations_from_ids_locked(data, entity_id, relation_types, true)
            }
            GraphDirection::Both => {
                let mut relations =
                    Self::relations_from_ids_locked(data, entity_id, relation_types, false);
                relations.extend(Self::relations_from_ids_locked(
                    data,
                    entity_id,
                    relation_types,
                    true,
                ));
                relations
            }
        }
    }

    fn relations_from_ids_locked(
        data: &GraphData,
        entity_id: &str,
        relation_types: Option<&[String]>,
        incoming: bool,
    ) -> Vec<EntityRelationRecord> {
        let keys = if incoming {
            data.incoming.get(entity_id)
        } else {
            data.outgoing.get(entity_id)
        };
        let Some(keys) = keys else {
            return Vec::new();
        };

        keys.iter()
            .filter_map(|other_id| {
                data.relations.values().find(|relation| {
                    if incoming {
                        relation.source == *other_id && relation.target == entity_id
                    } else {
                        relation.source == entity_id && relation.target == *other_id
                    }
                })
            })
            .filter(|relation| {
                relation_types
                    .map(|allowed| allowed.iter().any(|item| item == &relation.relation_type))
                    .unwrap_or(true)
            })
            .cloned()
            .collect()
    }
}

#[derive(Debug, Clone)]
pub struct RawRelation {
    pub source: String,
    pub target: String,
    pub relation_type: String,
    pub score: f32,
}

impl GraphData {
    fn resolve_entity_id(&self, entity_id_or_name: &str) -> Option<String> {
        if self.entities.contains_key(entity_id_or_name) {
            return Some(entity_id_or_name.to_string());
        }
        let key = normalize_name(entity_id_or_name);
        self.entity_lookup.get(&key).cloned()
    }

    fn upsert_entity(
        &mut self,
        entity: ExtractedEntity,
        memory_id: &str,
        metadata: Option<&serde_json::Value>,
        now: DateTime<Utc>,
    ) -> String {
        let normalized_name = normalize_name(&entity.name);
        let lookup_key = entity_lookup_key(&normalized_name, entity.entity_type);
        let entity_id = self
            .entity_lookup
            .get(&lookup_key)
            .cloned()
            .unwrap_or_else(|| ulid::Ulid::new().to_string());
        let record = self
            .entities
            .entry(entity_id.clone())
            .or_insert_with(|| EntityRecord {
                id: entity_id.clone(),
                name: entity.name.clone(),
                normalized_name: normalized_name.clone(),
                entity_type: entity.entity_type,
                aliases: Vec::new(),
                description: None,
                occurrence_count: 0,
                memory_count: 0,
                first_seen: now,
                last_seen: now,
                merged_from: Vec::new(),
                trust_score: 0.5,
                trust_rank: 0,
            });

        if record.name != entity.name {
            record.aliases.push(entity.name.clone());
            record.aliases.sort();
            record.aliases.dedup();
        }
        record.normalized_name = normalized_name.clone();
        record.entity_type = entity.entity_type;
        record.occurrence_count += 1;
        record.memory_count += 1;
        record.last_seen = now;
        if let Some(metadata) = metadata {
            if record.description.is_none() {
                record.description = metadata
                    .get("description")
                    .and_then(|value| value.as_str())
                    .map(|value| value.to_string());
            }
        }

        self.entity_lookup.insert(lookup_key, entity_id.clone());
        self.entity_lookup
            .entry(normalized_name.clone())
            .or_insert_with(|| entity_id.clone());
        self.memory_entities
            .entry(memory_id.to_string())
            .or_default()
            .insert(entity_id.clone());
        entity_id
    }

    fn upsert_relation(
        &mut self,
        source: &str,
        target: &str,
        relation_type: &str,
        weight: f32,
        co_occurrence_score: f32,
        memory_id: Option<&str>,
        now: DateTime<Utc>,
    ) -> EntityRelationRecord {
        let lookup_key = relation_lookup_key(source, target, relation_type);
        let relation_id = self
            .relation_lookup
            .get(&lookup_key)
            .cloned()
            .unwrap_or_else(|| ulid::Ulid::new().to_string());

        let entry = self
            .relations
            .entry(relation_id.clone())
            .or_insert_with(|| EntityRelationRecord {
                id: relation_id.clone(),
                source: source.to_string(),
                target: target.to_string(),
                relation_type: relation_type.to_string(),
                weight: 0.0,
                co_occurrence_score,
                support_count: 0,
                provenance: Vec::new(),
                created_at: now,
                updated_at: now,
            });

        entry.weight = (entry.weight + weight).min(10.0);
        entry.co_occurrence_score = co_occurrence_score.max(entry.co_occurrence_score);
        entry.support_count += 1;
        if let Some(memory_id) = memory_id {
            if !entry.provenance.iter().any(|item| item == memory_id) {
                entry.provenance.push(memory_id.to_string());
            }
        }
        entry.updated_at = now;

        self.relation_lookup.insert(lookup_key, relation_id.clone());
        self.outgoing
            .entry(source.to_string())
            .or_default()
            .insert(target.to_string());
        self.incoming
            .entry(target.to_string())
            .or_default()
            .insert(source.to_string());
        self.outgoing.entry(target.to_string()).or_default();
        self.incoming.entry(source.to_string()).or_default();

        entry.clone()
    }

    fn rebuild_indexes(&mut self) {
        self.entity_lookup.clear();
        self.outgoing.clear();
        self.incoming.clear();
        self.relation_lookup.clear();
        for entity in self.entities.values() {
            self.entity_lookup.insert(
                entity_lookup_key(&entity.normalized_name, entity.entity_type),
                entity.id.clone(),
            );
            self.entity_lookup
                .entry(entity.normalized_name.clone())
                .or_insert_with(|| entity.id.clone());
            for alias in &entity.aliases {
                self.entity_lookup
                    .entry(normalize_name(alias))
                    .or_insert_with(|| entity.id.clone());
            }
        }
        for relation in self.relations.values() {
            self.relation_lookup.insert(
                relation_lookup_key(&relation.source, &relation.target, &relation.relation_type),
                relation.id.clone(),
            );
            self.outgoing
                .entry(relation.source.clone())
                .or_default()
                .insert(relation.target.clone());
            self.incoming
                .entry(relation.target.clone())
                .or_default()
                .insert(relation.source.clone());
        }
    }

    fn relation_neighbors(&self, entity_id: &str) -> HashSet<String> {
        let mut neighbors = HashSet::new();
        if let Some(outgoing) = self.outgoing.get(entity_id) {
            neighbors.extend(outgoing.iter().cloned());
        }
        if let Some(incoming) = self.incoming.get(entity_id) {
            neighbors.extend(incoming.iter().cloned());
        }
        neighbors
    }

    fn relink_relation_neighbor(&mut self, from: &str, to: &str, neighbor: &str) {
        for relation in self.relations.values_mut() {
            if relation.source == from && relation.target == neighbor {
                relation.source = to.to_string();
            }
            if relation.target == from && relation.source == neighbor {
                relation.target = to.to_string();
            }
        }
    }

    fn remove_relations_for_entity(&mut self, entity_id: &str) {
        self.relations
            .retain(|_, relation| relation.source != entity_id && relation.target != entity_id);
    }
}

fn normalize_name(name: &str) -> String {
    name.split_whitespace()
        .map(|part| {
            part.trim_matches(|c: char| {
                matches!(c, ',' | '.' | ';' | ':' | '"' | '\'' | '(' | ')' | '[' | ']')
            })
        })
        .filter(|part| !part.is_empty())
        .map(|part| part.to_ascii_lowercase())
        .collect::<Vec<_>>()
        .join(" ")
}

fn entity_lookup_key(normalized_name: &str, entity_type: EntityType) -> String {
    format!("{}|{}", normalized_name, entity_type.as_str())
}

fn relation_lookup_key(source: &str, target: &str, relation_type: &str) -> String {
    format!("{}|{}|{}", source, target, relation_type)
}

fn is_common_word(value: &str) -> bool {
    COMMON_WORDS
        .iter()
        .any(|word| word.eq_ignore_ascii_case(value))
}

fn looks_like_organization(name: &str) -> bool {
    let lowered = name.to_ascii_lowercase();
    let org_markers = [
        " inc", " corp", " llc", " ltd", " company", " co", " labs", " lab", " systems",
        " studio", " platform", " foundation", " university", " institute", " agency", " team",
    ];
    name
        .chars()
        .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '-' || c == '_')
        || org_markers
            .iter()
            .any(|marker| lowered.ends_with(marker) || lowered.contains(marker))
}

fn looks_like_location(lowered: &str) -> bool {
    let location_markers = [
        " city", " town", " village", " province", " state", " country", " park", " valley",
        " mountain", " river", " lake", " bay", " beach", " street", " avenue",
    ];
    location_markers
        .iter()
        .any(|marker| lowered.ends_with(marker) || lowered.contains(marker))
}

fn looks_like_product(name: &str) -> bool {
    let lowered = name.to_ascii_lowercase();
    lowered.chars().any(|c| c.is_ascii_digit())
        || lowered.contains("model")
        || lowered.contains("platform")
        || lowered.contains("engine")
        || lowered.contains("sdk")
        || lowered.contains("api")
}

fn looks_like_person(name: &str) -> bool {
    let tokens: Vec<_> = name.split_whitespace().collect();
    (tokens.len() <= 3
        && tokens.iter().all(|token| {
            token
                .chars()
                .next()
                .is_some_and(|c| c.is_ascii_uppercase())
                || token.chars().all(|c| c.is_ascii_uppercase())
        }))
        || name.len() <= 8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn indexes_entities_relations_and_traversal() {
        let graph = EntityGraph::new();
        let view = graph
            .upsert_memory(
                "memory-1",
                "BELA works at SWAL and knows Leonardo in Bogota.",
                None,
            )
            .await
            .unwrap();

        assert!(view.total_relations > 0);
        let bela = graph.entity("BELA").await.expect("entity should exist");
        let neighbors = graph
            .entity_neighbors(&bela.id, 2, None, GraphDirection::Both)
            .await
            .unwrap();
        assert_eq!(neighbors.entity.id, bela.id);
        assert!(!neighbors.traversal.is_empty());
    }

    #[tokio::test]
    async fn merges_entities_and_preserves_aliases() {
        let graph = EntityGraph::new();
        graph
            .upsert_memory("memory-1", "Alice works at Acme", None)
            .await
            .unwrap();
        graph
            .upsert_memory("memory-2", "Alicia knows Bob", None)
            .await
            .unwrap();

        let entities = graph.all_entities().await;
        assert!(!entities.is_empty());
        let primary = graph.merge_entities("Alice", "Alicia").await.unwrap();
        assert!(primary.aliases.iter().any(|alias| alias.eq_ignore_ascii_case("Alicia")));
    }
}
