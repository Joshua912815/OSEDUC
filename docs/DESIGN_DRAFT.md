# OSeduc 设计方案草稿

本文档是给接力开发队友看的设计草稿，不是最终提交版设计方案。它的目的不是把所有细节一次写完，而是说明我们想做什么、现在已经做了什么、代码从哪里开始读、后续还需要补哪些模块。

## 1. 项目定位

OSeduc 是一个面向操作系统课程的 on-policy 智能教学平台。我们希望它不是一个静态资料站，而是一个围绕学生当前状态持续调整的学习系统：

- 用 OS 知识图谱组织课程主线。
- 用学生进度和交互记录维护学生模型。
- 用 policy engine 推荐下一步学习节点。
- 用受控 LLM tutor 基于课程材料进行讲解。
- 用练习题、实验反馈和 LLM 评测形成学习闭环。

当前项目优先支持 Rust OS 教学路线，主要参考 rCore-Tutorial-Book-v3 ch1-ch8。C 支持后续再考虑。

## 2. 核心设计思路

### 2.1 知识图谱驱动

课程内容被拆成 `KnowledgeNode`，节点之间用 `KnowledgeEdge` 表达先修和递进关系。每个节点都必须绑定来源材料，来源记录放在 `SourceReference` 里。导师回答不能凭空使用无来源上下文；如果请求指定了知识节点，后端会先从数据库取出对应 `RetrievalChunk`，再交给 LLM Gateway。

当前 seed 文件是 `data/knowledge/rcore-v3-rust-seed.json`，覆盖：

- ch1 应用执行环境
- ch2 批处理系统
- ch3 任务切换
- ch4 地址空间
- ch5 进程
- ch6 文件系统
- ch7 IPC / I/O 重定向
- ch8 并发

seed 中写的是我们自己的教学摘要和结构化上下文，不复制 rCore 原文长段落。每个 chunk 保留 rCore 章节 URL、citation label 和 license note。

### 2.2 On-policy 学习闭环

这里的 on-policy 指系统根据当前学生状态推荐下一步行动，然后根据学生完成情况更新状态，再重新推荐。首版闭环是：

1. 学生有一个 `StudentProfile`。
2. 每个知识节点有一个 `StudentNodeProgress`。
3. `oseduc-policy` 根据知识图谱和 progress 给出 `LearningPath`。
4. 学生向 tutor 提问或完成练习后，系统记录 interaction / feedback。
5. 后续把练习题成绩、LLM 评测结果和实验日志也写入学生模型。

当前 policy engine 是确定性的，故意先做简单可解释版本：未掌握先修时不推荐后续节点，`needs_review` 优先于新节点，掌握度足够高的节点跳过。

### 2.3 LLM 只作为受控教学层

前端不能直接调用 raw completion API。所有模型调用必须经过 `oseduc-llm` 的 `LlmGateway`：

- 统一读 env 配置。
- 支持 `mock` 和 `openai_compatible` provider。
- API key 用 redacted secret 类型包裹，不能 Debug/log 明文输出。
- system prompt 要求模型基于 rCore context 讲解，并返回 citation。
- 失败时返回结构化错误，不泄露 API key、完整 prompt 或私密学生日志。

当前默认 provider 是 `mock`，所以本地开发和测试不需要 API key。真实 LLM 联调只需要在本地 ignored `.env` 中配置：

```text
OSEDUC_LLM_PROVIDER=openai_compatible
OSEDUC_LLM_BASE_URL=https://api.openai.com/v1
OSEDUC_LLM_MODEL=your-model-name
OSEDUC_LLM_API_KEY=your-local-key
```

不要把 key 写进代码、README 示例以外的真实配置、测试输出或 issue。

## 3. 当前代码结构

Rust workspace 当前包含：

- `crates/oseduc-core`
  - 共享领域类型。
  - 重点读 `knowledge.rs`、`student.rs`、`tutor.rs`。
- `crates/oseduc-llm`
  - LLM provider trait、mock provider、OpenAI-compatible provider。
  - 重点读 `config.rs` 和 `provider.rs`。
- `crates/oseduc-store`
  - Postgres 配置、migration、repository、seed 校验。
  - 重点读 `repository.rs`、`seed.rs`、`migrations/`。
- `crates/oseduc-policy`
  - 当前学习路径推荐逻辑。
- `crates/oseduc-api`
  - Axum HTTP API、runtime config、router。
  - 重点读 `config.rs`、`router.rs`、`main.rs`。

本地开发入口：

```bash
docker compose up -d postgres
export OSEDUC_DATABASE_URL=postgres://oseduc:oseduc_dev_password@127.0.0.1:5432/oseduc
export OSEDUC_AUTO_MIGRATE=true
cargo run -p oseduc-api
```

## 4. 当前已经实现的能力

### 4.1 Postgres 知识图谱

已经有 migration：

- `source_references`
- `knowledge_nodes`
- `knowledge_edges`
- `retrieval_chunks`
- `student_profiles`
- `student_node_progress`
- `tutor_interactions`
- `tutor_interaction_feedback`

内置 seed 可通过 admin endpoint 导入：

```bash
export OSEDUC_ENABLE_ADMIN_SEED=true
export OSEDUC_ADMIN_TOKEN=replace-with-local-admin-token
curl -X POST \
  -H "Authorization: Bearer $OSEDUC_ADMIN_TOKEN" \
  http://127.0.0.1:3000/v1/admin/knowledge/seed
```

admin seed 默认关闭；开启时必须提供 token。

### 4.2 知识图谱 API

已有 endpoint：

- `GET /v1/knowledge/nodes`
- `GET /v1/knowledge/nodes/{id}`
- `GET /v1/knowledge/nodes/{id}/neighbors`
- `GET /v1/sources`

### 4.3 学生模型和学习路径

已有 endpoint：

- `GET /v1/students/{student_id}/profile`
- `PUT /v1/students/{student_id}/profile`
- `GET /v1/students/{student_id}/progress`
- `PUT /v1/students/{student_id}/progress/{node_id}`
- `GET /v1/students/{student_id}/learning-path`

progress 支持 `not_started`、`in_progress`、`needs_review`、`mastered`，掌握度是 0 到 100。

### 4.4 Tutor chat 和引用闭环

已有 endpoint：

- `POST /v1/tutor/chat`

请求可以带：

```json
{
  "message": "Explain address spaces",
  "student_id": "student-1",
  "knowledge_node_ids": ["ch4-address-space"]
}
```

后端会：

1. 根据 `knowledge_node_ids` 查 source-grounded context。
2. 交给 LLM Gateway。
3. 返回 answer、provider、citations、safety_flags。
4. 写入 `tutor_interactions`。
5. 在响应里返回 `interaction_id`。

如果请求了不存在的 node，不会让 LLM 猜，而是返回 `knowledge_context_missing`。

### 4.5 Tutor interaction history 和反馈

已有 endpoint：

- `GET /v1/students/{student_id}/tutor/interactions?limit=20`
- `PUT /v1/tutor/interactions/{interaction_id}/feedback`

默认 `OSEDUC_LOG_STUDENT_MESSAGES=false`，所以 history 只保存 provider、知识节点、citation、safety flag、时间戳等元数据，不保存学生原始问题文本。这样后续做学习分析时有足够结构化数据，同时降低隐私风险。

反馈格式：

```json
{
  "helpful": true,
  "difficulty": "just_right",
  "feedback_text": "clear citations"
}
```

## 5. 已验证内容

当前后端已跑过：

```bash
cargo fmt --check
cargo test
```

测试覆盖：

- mock provider 不要求 API key。
- openai-compatible provider 缺 API key 会失败。
- secret Debug/Display 不暴露 key。
- public config 不暴露 API key、database credential、admin token。
- knowledge node/source API。
- admin seed 默认关闭，开启后需要 token。
- student profile/progress/learning-path。
- tutor chat 返回 citation 和 interaction_id。
- tutor history 默认不返回原始 message。
- tutor feedback 可写入。

也做过本地 Postgres smoke：

- 迁移临时数据库。
- 导入 rCore seed。
- 调用 tutor chat。
- 查询 interaction history。
- 写入 feedback。
- 再查 history 确认 feedback 挂载成功。

## 6. 下一阶段：练习题与 LLM 评测

用户明确希望 OSeduc 有针对性出练习题功能，并且可以由 LLM 负责评测题目。这应该是下一阶段的重点。

### 6.1 练习题模块目标

练习题不是独立题库，而是 knowledge graph 和 policy engine 的一部分。每道题应绑定：

- 对应知识节点。
- 题型。
- 难度。
- 来源和 license。
- 评分规则。
- 是否允许 LLM 评测。
- 是否允许 LLM 生成提示。

首版建议支持三类题：

1. 概念选择/判断题
   - 适合自动评分。
   - 用于快速检测知识节点掌握情况。
2. 简答题
   - 由 LLM 按 rubric 评测。
   - 要求返回分数、错因、建议复习节点、引用来源。
3. 实验诊断题
   - 输入错误日志、运行结果或学生解释。
   - LLM 结合 rubric 和知识节点判断学生卡在哪里。

### 6.2 建议新增领域类型

可在 `oseduc-core` 增加：

- `Exercise`
  - `id`
  - `node_id`
  - `title`
  - `exercise_type`
  - `difficulty`
  - `prompt`
  - `rubric`
  - `source_id`
  - `llm_grading_policy`
- `ExerciseAttempt`
  - `id`
  - `exercise_id`
  - `student_id`
  - `answer`
  - `status`
  - `submitted_at`
- `ExerciseEvaluation`
  - `attempt_id`
  - `score`
  - `max_score`
  - `mastery_delta`
  - `feedback`
  - `misconceptions`
  - `recommended_node_ids`
  - `citations`
  - `safety_flags`

### 6.3 建议新增数据库表

可在 `oseduc-store/migrations` 增加：

- `exercises`
- `exercise_attempts`
- `exercise_evaluations`

注意：学生答案可能包含隐私或作业内容。是否保存完整答案要加配置，类似 `OSEDUC_LOG_STUDENT_MESSAGES`。建议默认保存 attempt metadata 和 evaluation，完整 answer 可先保存，但后续需要教师/课程策略明确。

### 6.4 建议新增 API

首版 endpoint：

- `GET /v1/knowledge/nodes/{id}/exercises`
- `GET /v1/exercises/{exercise_id}`
- `POST /v1/exercises/{exercise_id}/attempts`
- `GET /v1/students/{student_id}/exercise-attempts`
- `POST /v1/exercise-attempts/{attempt_id}/evaluate`

为了方便前端，`POST /v1/exercises/{exercise_id}/attempts` 可以选择 `evaluate_now=true`，提交后立即走 LLM 评测。

### 6.5 LLM 评测设计

不要让 LLM 自由发挥评分。需要设计 `Exercise Grading Gateway`，复用 `oseduc-llm`，但 prompt 和返回结构独立于 tutor chat。

评测输入应包括：

- 题目 prompt。
- 标准 rubric。
- 学生答案。
- 相关 knowledge node。
- rCore source-grounded context。
- 学术诚信规则。

评测输出必须是结构化 JSON，例如：

```json
{
  "score": 7,
  "max_score": 10,
  "feedback": "The answer explains page tables but misses isolation between user and kernel spaces.",
  "misconceptions": ["confuses virtual address with physical address"],
  "recommended_node_ids": ["ch4-address-space"],
  "citations": [
    {
      "label": "rCore v3 ch4",
      "source": "https://rcore-os.cn/rCore-Tutorial-Book-v3/chapter4/index.html",
      "node_id": "ch4-address-space"
    }
  ],
  "safety_flags": ["source_grounded_context"]
}
```

后端收到 LLM 返回后要做校验：

- 分数必须在 `[0, max_score]`。
- citations 不能为空，除非题目明确不需要来源。
- recommended node 必须存在。
- feedback 不能包含完整标准答案泄露。
- provider error 不能泄露 key、raw prompt 或学生隐私。

### 6.6 与学习路径的关系

练习评测完成后，应更新 `student_node_progress`：

- 高分：提高 mastery score。
- 中等：标为 `in_progress`。
- 低分或关键误区：标为 `needs_review`。
- misconception 映射到 common misconceptions，推动下一步推荐。

Policy engine 下一步要支持：

- 如果学生刚完成 tutor chat 但没有练习验证，推荐对应练习。
- 如果学生练习低分，推荐先修节点或更简单题。
- 如果学生连续高分，推荐下一章或更难题。

## 7. 前端接力建议

当前还没有前端。队友如果接前端，建议先做非常薄的开发 UI，而不是复杂视觉：

- 节点列表页：展示 ch1-ch8 节点。
- 节点详情页：展示 summary、source、neighbors。
- 学生进度页：手动设置 progress，观察 learning path 变化。
- Tutor 页：选择知识节点，输入问题，展示 answer/citations/safety_flags/interaction_id。
- Tutor history 页：展示 interaction metadata 和 feedback。
- 练习题页：下一阶段接 exercises API。

前端不要保存 API key。真实 LLM key 只放后端环境变量。

## 8. 非本队来源与 license 边界

本项目目前涉及以下外部来源：

- rCore-Tutorial-Book-v3
  - 用作 Rust OS 教学主线参考。
  - seed 中保留章节 URL、citation label、license note。
  - 不复制大段正文，不把 rCore 内容声称为 OSeduc 原创。
- `xuhengyi/spec-driven-rust-os`
  - 本地 reference，GPLv3。
  - 不得直接复制 GPLv3 代码进非 GPL 项目，除非做 license 兼容审查。
- `xuhengyi/spec-driven-c-os`
  - 本地 reference，MIT。
  - 直接复用时必须保留 copyright 和 permission notice。
- `xuhengyi/fm-agent-tgrcore-reproduction`
  - 本地 research reference。
  - 仓库级 license 需要进一步澄清，暂不直接并入项目代码。

详细 credit 见 `REFERENCE_CREDITS.md`。后续任何 copied/adapted material 都要记录 source path、commit、license 和使用方式。

## 9. 当前遗留问题

### 9.1 需要真实 LLM 联调

目前 mock provider 已完整可测，OpenAI-compatible provider 已有代码，但还没有用真实 key 做端到端质量验证。真实联调前不要把 key 发到聊天里，放本地 `.env`。

### 9.2 需要练习题 schema 和 seed

下一阶段应先手写少量 ch4/ch5 练习题 seed，覆盖地址空间、页表、进程、fork/wait。题目必须有 rubric 和 source metadata。

### 9.3 需要 LLM 评测 guardrail

LLM 评测要比 tutor chat 更严格，因为它会影响学生模型。需要结构化输出校验、分数范围校验、citation 校验、recommended node 校验。

### 9.4 需要迁移策略

本地开发数据库如果已经应用过同名 migration，修改 migration 会触发 checksum mismatch。已经提交过的 migration 不要再改，新增表或字段应继续追加新的 timestamp migration。

### 9.5 需要更完整的设计方案终稿

最终设计方案还要补：

- 系统架构图。
- 数据流图。
- 关键 API 表格。
- LLM prompt 和 guardrail 说明。
- 研发过程中的问题与解决方法。
- 更正式的非本队来源说明。
- 运行截图或 smoke 测试记录。

## 10. 接力开发优先级

建议下一位队友按这个顺序做：

1. 先拉最新 `main`，跑 `cargo fmt --check && cargo test`。
2. 本地启动 Postgres，导入 seed，确认 `/v1/tutor/chat` 返回 citation 和 interaction_id。
3. 新增 exercise domain 类型和 migration。
4. 手写 ch4/ch5 少量练习题 seed。
5. 增加 exercise list/detail/attempt API。
6. 增加 LLM grading trait 和 mock grading provider。
7. 用 mock grading provider 跑通 attempt -> evaluation -> progress update。
8. 再考虑真实 LLM 评测联调。

这个顺序能保持后端一直可运行、可测试，也能避免先做复杂前端导致业务闭环不清楚。
