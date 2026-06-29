const state = {
  nodes: [],
  nodeDetails: new Map(),
  progress: new Map(),
  selectedNodeId: null,
  studentId: "student-demo",
};

const elements = {
  apiStatus: document.querySelector("#apiStatus"),
  refreshButton: document.querySelector("#refreshButton"),
  studentIdInput: document.querySelector("#studentIdInput"),
  nodeCount: document.querySelector("#nodeCount"),
  nodeList: document.querySelector("#nodeList"),
  learningMap: document.querySelector("#learningMap"),
  mapStatus: document.querySelector("#mapStatus"),
  nodeTitle: document.querySelector("#nodeTitle"),
  nodeChapter: document.querySelector("#nodeChapter"),
  nodeSummary: document.querySelector("#nodeSummary"),
  objectivesList: document.querySelector("#objectivesList"),
  misconceptionsList: document.querySelector("#misconceptionsList"),
  sourceBox: document.querySelector("#sourceBox"),
  displayNameInput: document.querySelector("#displayNameInput"),
  depthSelect: document.querySelector("#depthSelect"),
  goalInput: document.querySelector("#goalInput"),
  profileStatus: document.querySelector("#profileStatus"),
  saveProfileButton: document.querySelector("#saveProfileButton"),
  recommendationList: document.querySelector("#recommendationList"),
  pathStatus: document.querySelector("#pathStatus"),
  progressStatus: document.querySelector("#progressStatus"),
  progressSelect: document.querySelector("#progressSelect"),
  masteryInput: document.querySelector("#masteryInput"),
  progressNotesInput: document.querySelector("#progressNotesInput"),
  saveProgressButton: document.querySelector("#saveProgressButton"),
  questionInput: document.querySelector("#questionInput"),
  askTutorButton: document.querySelector("#askTutorButton"),
  tutorAnswer: document.querySelector("#tutorAnswer"),
  tutorStatus: document.querySelector("#tutorStatus"),
  adminTokenInput: document.querySelector("#adminTokenInput"),
  seedButton: document.querySelector("#seedButton"),
  toast: document.querySelector("#toast"),
};

function api(path, options = {}) {
  const headers = {
    ...(options.body ? { "content-type": "application/json" } : {}),
    ...(options.headers || {}),
  };
  return fetch(`/api${path}`, {
    ...options,
    headers,
  }).then(async (response) => {
    const text = await response.text();
    const payload = text ? JSON.parse(text) : null;
    if (!response.ok) {
      const message = payload?.message || response.statusText;
      throw new Error(message);
    }
    return payload;
  });
}

function currentStudentId() {
  const value = elements.studentIdInput.value.trim();
  return value || "student-demo";
}

function chapterNumber(chapter) {
  const match = chapter.match(/\d+/);
  return match ? Number(match[0]) : 999;
}

function sortNodes(nodes) {
  return [...nodes].sort((left, right) => {
    const chapterDiff = chapterNumber(left.chapter) - chapterNumber(right.chapter);
    return chapterDiff || left.id.localeCompare(right.id);
  });
}

function progressFor(nodeId) {
  return state.progress.get(nodeId) || {
    status: "not_started",
    mastery_score: 0,
    notes: "",
  };
}

function progressLabel(status) {
  return status.replaceAll("_", " ");
}

function showToast(message) {
  elements.toast.textContent = message;
  elements.toast.classList.add("visible");
  window.clearTimeout(showToast.timer);
  showToast.timer = window.setTimeout(() => {
    elements.toast.classList.remove("visible");
  }, 2600);
}

function setApiStatus(message, ok = true) {
  elements.apiStatus.textContent = message;
  elements.apiStatus.style.color = ok ? "var(--muted)" : "var(--accent)";
}

function renderNodeList() {
  elements.nodeCount.textContent = `${state.nodes.length} nodes`;
  elements.nodeList.replaceChildren(
    ...state.nodes.map((node) => {
      const progress = progressFor(node.id);
      const button = document.createElement("button");
      button.type = "button";
      button.className = `node-button ${node.id === state.selectedNodeId ? "active" : ""}`;
      button.addEventListener("click", () => selectNode(node.id));

      const title = document.createElement("span");
      title.className = "node-title";
      title.textContent = node.title;

      const status = document.createElement("span");
      status.className = `progress-pill ${progress.status}`;
      status.textContent = progressLabel(progress.status);

      const subtitle = document.createElement("span");
      subtitle.className = "node-subtitle";
      subtitle.textContent = `${node.chapter} · ${node.summary}`;

      button.append(title, status, subtitle);
      return button;
    }),
  );
}

function renderMap() {
  elements.mapStatus.textContent = state.nodes.length ? "Rust OS ch1-ch8" : "No data";
  elements.learningMap.replaceChildren(
    ...state.nodes.map((node) => {
      const progress = progressFor(node.id);
      const step = document.createElement("button");
      step.type = "button";
      step.className = `map-step ${node.id === state.selectedNodeId ? "active" : ""}`;
      step.addEventListener("click", () => selectNode(node.id));

      const title = document.createElement("span");
      title.className = "map-step-title";
      title.textContent = node.title;

      const badge = document.createElement("span");
      badge.className = `progress-pill ${progress.status}`;
      badge.textContent = `${progress.mastery_score || 0}`;

      step.append(title, badge);
      return step;
    }),
  );
}

async function selectNode(nodeId) {
  state.selectedNodeId = nodeId;
  renderNodeList();
  renderMap();
  await loadNodeDetail(nodeId);
  renderProgressEditor();
}

async function loadNodeDetail(nodeId) {
  let detail = state.nodeDetails.get(nodeId);
  if (!detail) {
    detail = await api(`/v1/knowledge/nodes/${encodeURIComponent(nodeId)}`);
    state.nodeDetails.set(nodeId, detail);
  }

  const { node, source, retrieval_chunks: chunks = [] } = detail;
  elements.nodeTitle.textContent = node.title;
  elements.nodeChapter.textContent = node.chapter;
  elements.nodeSummary.textContent = node.summary;
  renderList(elements.objectivesList, node.learning_objectives);
  renderList(elements.misconceptionsList, node.common_misconceptions);

  const firstChunk = chunks[0];
  elements.sourceBox.innerHTML = "";
  const link = document.createElement("a");
  link.href = source.url;
  link.target = "_blank";
  link.rel = "noreferrer";
  link.textContent = source.title;
  elements.sourceBox.append(link);
  elements.sourceBox.append(
    document.createTextNode(
      ` · ${firstChunk?.citation_label || "citation pending"} · ${source.license_note}`,
    ),
  );
}

function renderList(target, values = []) {
  target.replaceChildren(
    ...(values.length ? values : ["No entries yet"]).map((value) => {
      const item = document.createElement("li");
      item.textContent = value;
      return item;
    }),
  );
}

function renderProgressEditor() {
  if (!state.selectedNodeId) {
    return;
  }
  const progress = progressFor(state.selectedNodeId);
  elements.progressSelect.value = progress.status;
  elements.masteryInput.value = String(progress.mastery_score ?? 0);
  elements.progressNotesInput.value = progress.notes || "";
  elements.progressStatus.textContent = `${progressLabel(progress.status)} · ${progress.mastery_score ?? 0}`;
}

async function loadProfile() {
  const profile = await api(`/v1/students/${encodeURIComponent(state.studentId)}/profile`);
  elements.displayNameInput.value = profile.display_name || "";
  elements.goalInput.value = profile.learning_goal || "";
  elements.depthSelect.value = profile.preferred_depth || "balanced";
  elements.profileStatus.textContent = "Loaded";
}

async function saveProfile() {
  const body = {
    display_name: elements.displayNameInput.value.trim() || null,
    learning_goal: elements.goalInput.value.trim() || null,
    preferred_depth: elements.depthSelect.value,
  };
  await api(`/v1/students/${encodeURIComponent(state.studentId)}/profile`, {
    method: "PUT",
    body: JSON.stringify(body),
  });
  elements.profileStatus.textContent = "Saved";
  showToast("Profile saved");
}

async function loadProgress() {
  const progress = await api(`/v1/students/${encodeURIComponent(state.studentId)}/progress`);
  state.progress = new Map(progress.map((entry) => [entry.node_id, entry]));
  renderNodeList();
  renderMap();
  renderProgressEditor();
}

async function saveProgress() {
  if (!state.selectedNodeId) {
    showToast("Select a knowledge node first");
    return;
  }
  const body = {
    status: elements.progressSelect.value,
    mastery_score: Number(elements.masteryInput.value),
    notes: elements.progressNotesInput.value.trim() || null,
  };
  const progress = await api(
    `/v1/students/${encodeURIComponent(state.studentId)}/progress/${encodeURIComponent(
      state.selectedNodeId,
    )}`,
    {
      method: "PUT",
      body: JSON.stringify(body),
    },
  );
  state.progress.set(progress.node_id, progress);
  renderNodeList();
  renderMap();
  renderProgressEditor();
  await loadLearningPath();
  showToast("Progress saved");
}

async function loadLearningPath() {
  const path = await api(
    `/v1/students/${encodeURIComponent(state.studentId)}/learning-path?limit=5`,
  );
  elements.pathStatus.textContent = `${path.completed_nodes}/${path.total_nodes} mastered`;
  elements.recommendationList.replaceChildren(
    ...path.recommendations.map((recommendation) => {
      const button = document.createElement("button");
      button.type = "button";
      button.className = "recommendation-button";
      button.addEventListener("click", () => selectNode(recommendation.node.id));

      const title = document.createElement("span");
      title.className = "node-title";
      title.textContent = recommendation.node.title;

      const priority = document.createElement("span");
      priority.className = "badge";
      priority.textContent = `P${recommendation.priority}`;

      const reason = document.createElement("span");
      reason.className = "node-subtitle";
      reason.textContent = recommendation.reason;

      button.append(title, priority, reason);
      return button;
    }),
  );
  if (!path.recommendations.length) {
    elements.recommendationList.textContent = "No open recommendations.";
  }
}

async function askTutor() {
  if (!state.selectedNodeId) {
    showToast("Select a knowledge node first");
    return;
  }
  elements.tutorStatus.textContent = "Asking...";
  const response = await api("/v1/tutor/chat", {
    method: "POST",
    body: JSON.stringify({
      message: elements.questionInput.value.trim(),
      student_id: state.studentId,
      knowledge_node_ids: [state.selectedNodeId],
    }),
  });
  elements.tutorAnswer.textContent = response.answer;
  const citationList = document.createElement("div");
  citationList.className = "citation-list";
  citationList.textContent = response.citations
    .map((citation) => `${citation.label}: ${citation.source}`)
    .join("\n");
  elements.tutorAnswer.append(citationList);
  elements.tutorStatus.textContent = response.provider;
}

async function seedGraph() {
  const token = elements.adminTokenInput.value.trim();
  if (!token) {
    showToast("Admin token required");
    return;
  }
  const result = await api("/v1/admin/knowledge/seed", {
    method: "POST",
    headers: { authorization: `Bearer ${token}` },
  });
  showToast(`Seeded ${result.nodes} nodes`);
  await refreshAll();
}

async function refreshAll() {
  state.studentId = currentStudentId();
  setApiStatus("Loading backend data...");
  const [health, nodes] = await Promise.all([api("/healthz"), api("/v1/knowledge/nodes")]);
  state.nodes = sortNodes(nodes);
  state.selectedNodeId = state.selectedNodeId || state.nodes[0]?.id || null;
  setApiStatus(`Backend ${health.status}`);
  await Promise.all([loadProfile(), loadProgress()]);
  renderNodeList();
  renderMap();
  if (state.selectedNodeId) {
    await loadNodeDetail(state.selectedNodeId);
  }
  await loadLearningPath();
}

async function run(action) {
  try {
    await action();
  } catch (error) {
    console.error(error);
    setApiStatus(error.message, false);
    showToast(error.message);
  }
}

elements.refreshButton.addEventListener("click", () => run(refreshAll));
elements.studentIdInput.addEventListener("change", () => run(refreshAll));
elements.saveProfileButton.addEventListener("click", () => run(saveProfile));
elements.saveProgressButton.addEventListener("click", () => run(saveProgress));
elements.askTutorButton.addEventListener("click", () => run(askTutor));
elements.seedButton.addEventListener("click", () => run(seedGraph));

run(refreshAll);
