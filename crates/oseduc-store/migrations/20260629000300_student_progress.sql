CREATE TABLE student_profiles (
    student_id TEXT PRIMARY KEY,
    display_name TEXT,
    learning_goal TEXT,
    preferred_depth TEXT NOT NULL DEFAULT 'balanced',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE student_node_progress (
    student_id TEXT NOT NULL REFERENCES student_profiles(student_id) ON DELETE CASCADE,
    node_id TEXT NOT NULL REFERENCES knowledge_nodes(id) ON DELETE CASCADE,
    status TEXT NOT NULL,
    mastery_score SMALLINT NOT NULL DEFAULT 0 CHECK (mastery_score >= 0 AND mastery_score <= 100),
    notes TEXT,
    last_interaction_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (student_id, node_id)
);

CREATE INDEX student_node_progress_student_id_idx ON student_node_progress (student_id);
CREATE INDEX student_node_progress_node_id_idx ON student_node_progress (node_id);
CREATE INDEX student_node_progress_status_idx ON student_node_progress (status);
