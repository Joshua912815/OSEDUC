CREATE TABLE tutor_interactions (
    id BIGSERIAL PRIMARY KEY,
    student_id TEXT REFERENCES student_profiles(student_id) ON DELETE SET NULL,
    provider TEXT NOT NULL,
    knowledge_node_ids TEXT[] NOT NULL DEFAULT '{}',
    citation_labels TEXT[] NOT NULL DEFAULT '{}',
    citation_sources TEXT[] NOT NULL DEFAULT '{}',
    citation_node_ids TEXT[] NOT NULL DEFAULT '{}',
    safety_flags TEXT[] NOT NULL DEFAULT '{}',
    message_logged BOOLEAN NOT NULL DEFAULT false,
    message TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX tutor_interactions_student_id_created_at_idx
    ON tutor_interactions (student_id, created_at DESC);

CREATE TABLE tutor_interaction_feedback (
    interaction_id BIGINT PRIMARY KEY REFERENCES tutor_interactions(id) ON DELETE CASCADE,
    helpful BOOLEAN,
    difficulty TEXT,
    feedback_text TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
