CREATE TABLE source_references (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    url TEXT NOT NULL,
    source_kind TEXT NOT NULL,
    license_note TEXT NOT NULL,
    retrieved_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE knowledge_nodes (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    chapter TEXT NOT NULL,
    kind TEXT NOT NULL,
    summary TEXT NOT NULL,
    learning_objectives TEXT[] NOT NULL DEFAULT '{}',
    common_misconceptions TEXT[] NOT NULL DEFAULT '{}',
    source_id TEXT NOT NULL REFERENCES source_references(id) ON DELETE RESTRICT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE knowledge_edges (
    from_node_id TEXT NOT NULL REFERENCES knowledge_nodes(id) ON DELETE CASCADE,
    to_node_id TEXT NOT NULL REFERENCES knowledge_nodes(id) ON DELETE CASCADE,
    relation TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (from_node_id, to_node_id, relation)
);

CREATE TABLE retrieval_chunks (
    id TEXT PRIMARY KEY,
    node_id TEXT NOT NULL REFERENCES knowledge_nodes(id) ON DELETE CASCADE,
    source_id TEXT NOT NULL REFERENCES source_references(id) ON DELETE RESTRICT,
    original_summary TEXT NOT NULL,
    citation_label TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX knowledge_nodes_chapter_idx ON knowledge_nodes (chapter);
CREATE INDEX knowledge_nodes_source_id_idx ON knowledge_nodes (source_id);
CREATE INDEX knowledge_edges_from_node_id_idx ON knowledge_edges (from_node_id);
CREATE INDEX knowledge_edges_to_node_id_idx ON knowledge_edges (to_node_id);
CREATE INDEX retrieval_chunks_node_id_idx ON retrieval_chunks (node_id);
CREATE INDEX retrieval_chunks_source_id_idx ON retrieval_chunks (source_id);
