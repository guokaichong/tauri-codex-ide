# TauriCodexIDE — 轻量化AI代码IDE（对标Codex，国内环境）
> 项目定位：本地轻量GUI客户端，AI推理使用国内云端API，编译打包全部放到 **GitHub Actions 免费 Windows 云端流水线**；适配 N95 / 12~16G Windows10 LTSC2021 低配笔记本，产出 NSIS 单安装包（当前用户安装，免管理员）。
>
> 📖 环境搭建、双远端推送、云端构建、发版流程与踩坑全记录见 **[docs/开发与部署手册.md](docs/开发与部署手册.md)**。

## 核心架构思路
- **本地端**：Tauri2 + Rust + Svelte + 精简Monaco Editor
  只做GUI、本地文件读写、终端白名单命令执行、API请求转发。**本地不跑大模型推理**
- **AI算力**：优先国内云端API（火山方舟 Agent Plan / Coding Plan / 按量 + MiniMax开放平台 + DeepSeek API，全部兼容OpenAI协议）；可一键降级本地Ollama(127.0.0.1:11434)。内置 **7 个供应商 · 30+ 模型**，详见下方「AI 模型接入」章节
- **编译打包**：Rust编译、Tauri打包、资源打包全部在 GitHub Actions 的 `windows-latest` 免费云端执行（推送 `v*` 标签自动构建并发布 Release，见 [开发与部署手册](docs/开发与部署手册.md) 第 4、5 章），本机仅写代码。Gitee 仓库仅作镜像（Gitee 免费云端无 Windows 构建机）。
- **安全策略**：不会自动上传完整项目源码；**仅用户手动确认后，选中的代码片段才发送到云端API**；API密钥保存在本地，禁止提交仓库。

## 硬件&系统约束
- 宿主系统：Windows10 LTSC2021
- CPU：N95
- 内存：12~16GB
- 性能目标：客户端空闲内存 <600MB；禁用复杂动画、减少轮询降低CPU占用
- 网络：中国大陆网络，无海外依赖、不调用chatgpt.com、不引入海外CDN

## 技术栈强制要求
- 后端：Tauri2、Rust
- 前端：Svelte + Vite + 精简 Monaco Editor
- 禁止：Electron、React全家桶重型框架、OpenAI官方接口、海外CDN静态资源
- 存储：本地JSON持久化配置（API Key、模型、代理、工作目录）

## AI 模型接入（已真机联调）

> 完整版文档（含全部密钥、模型实测结果、校验脚本）：
> `e:\Code\AI_Projects\VidProj\AI_MODEL_CONFIG.md` / `AI_MODEL_CONFIG.txt`
> ⚠️ 该文档明文保存密钥，已加入 `.gitignore`，**严禁上传公共仓库**。

### 供应商与端点（内置 7 个，下拉即选）

| 供应商（下拉项） | Base URL | 默认对话模型 | 默认 FIM 模型 |
| --- | --- | --- | --- |
| 火山方舟 Agent Plan（订阅） | `https://ark.cn-beijing.volces.com/api/plan/v3` | `doubao-seed-evolving` | `deepseek-v4.1-flash` |
| 火山方舟 Coding Plan（订阅） | `https://ark.cn-beijing.volces.com/api/coding/v3` | `doubao-seed-evolving` | `deepseek-v4-flash` |
| 火山方舟 Ark（按量） | `https://ark.cn-beijing.volces.com/api/v3` | `doubao-seed-evolving` | `doubao-seed-evolving` |
| MiniMax 开放平台 | `https://api.minimax.cn/v1` | `MiniMax-M3` | `MiniMax-M2.7-highspeed` |
| DeepSeek | `https://api.deepseek.com/v1` | `deepseek-chat` | `deepseek-chat` |
| 本地 Ollama | `http://127.0.0.1:11434/v1` | `qwen2.5-coder:7b` | `qwen2.5-coder:7b` |
| 自定义 OpenAI 兼容 | 手动填写 | 手动填写 | 手动填写 |

**火山方舟三套通道互不相通**，同一把密钥换端点一定 `401`：

| 通道 | 端点 | 用哪把密钥 |
| --- | --- | --- |
| Agent Plan（订阅） | `/api/plan/v3` | Agent Plan 专属密钥 |
| Coding Plan（订阅） | `/api/coding/v3` | Coding Plan 专属密钥 |
| 按量付费 | `/api/v3` | 普通按量 API Key |

> Coding Plan 的正确端点是 `/api/coding/v3`，**不是** `/api/plan/v3`；后者实测返回 `401 AuthenticationError`。

### 模型清单（实测 2026-09）

- **Agent Plan**：14 个模型全部 200 OK，含 `doubao-seed-evolving`、`deepseek-v4.1-flash`、`deepseek-v4-pro`、`kimi-k2.8-preview`、`kimi-k2.7-code`、`kimi-k3`、`minimax-m3`、`glm-5.3` / `glm-5.3-flash`、`doubao-seed-2.1-turbo` / `2.0-lite` / `2.0-mini`、`ark-code-latest`
- **Coding Plan**：7 个可用；`deepseek-v4.1-flash` 返回 `404 UnsupportedModel`（该模型不支持 coding plan），已在注册表中剔除
- **按量**：文本仅 `doubao-seed-evolving`；图像 `doubao-seedream-5-0-pro-260628`、视频 `doubao-seedance-1-5-pro-251215` 走此通道
- **MiniMax**：`MiniMax-M3` 等全系可用，返回带 `<think>` 思考标签
- **DeepSeek**：`deepseek-chat` 可用（服务端返回 `deepseek-flash`）

### 两个特殊处理（后端已实现）

1. **FIM 模拟**：火山方舟与 MiniMax **都没有原生 `/completions` 接口**。`api_gateway.rs` 检测到这几个供应商时，自动改用 `/chat/completions` 组装补全提示词；界面会提示「用对话接口模拟」，属正常现象。
2. **思考标签剥离**：MiniMax 全系 + Kimi 系列 + `reasoner`/`r1`/`thinking` 模型会返回 `<think>…</think>`（部分字段为 `reasoning_content`）。后端 `strip_thinking()` 统一剥离，未闭合的截断思考一并丢弃，避免污染代码输出。

### 本机部署（配置落盘位置）

配置文件：

```
%APPDATA%\TauriCodexIDE\local-settings.json
```

默认完整路径：

```
C:\Users\Administrator\AppData\Roaming\TauriCodexIDE\local-settings.json
```

内容结构：

```json
{
  "provider": "ark-agent-plan",
  "api_base": "https://ark.cn-beijing.volces.com/api/plan/v3",
  "api_key": "<在此填入 Agent Plan 密钥>",
  "chat_model": "doubao-seed-evolving",
  "fim_model": "deepseek-v4.1-flash",
  "timeout_secs": 180,
  "proxy": "",
  "workspace_dir": ""
}
```

**图形界面方式（推荐，无需手写 JSON）**：

1. 打开 IDE → 设置（Settings）
2. 「供应商」下拉选目标通道（如 **火山方舟 Agent Plan（订阅）**）
3. 「对话模型」「FIM 补全模型」下拉选模型
4. API Key 粘贴对应密钥
5. 保存（Base URL 会随供应商自动填好）

**部署校验**：

1. 保存后发一条测试对话，确认能出结果
2. 逐个切换模型各测一条，确认无 `401`（密钥与端点不匹配）/ `404`（模型未开通或不支持该通道）/ `429`（RPM 限流）
3. 确认 `local-settings.json` 未被提交到仓库（已由 `.gitignore` 排除）

错误码速查：

| 返回 | 含义 | 处理 |
| --- | --- | --- |
| `401 AuthenticationError` | 密钥与端点不匹配 | 对照上文三通道表换端点 |
| `404 InvalidEndpointOrModel.NotFound` | 鉴权通过，模型名错或未开通 | 换模型 ID / 控制台开通 |
| `404 UnsupportedModel` | 模型不支持当前套餐通道 | 换通道 |
| `429 ModelAccountRpmRateLimitExceeded` | 触发 RPM 限流 | 降并发 / 稍后重试 |

## 功能清单
### 本地IDE客户端模块
1. 项目工作区 & 文件树：本地文件夹导入、多文件浏览读写；**沙箱限制，禁止跨目录读取系统敏感文件**
2. 精简代码编辑器：Monaco Editor裁剪版，语法高亮、代码选中唤起AI对话
3. AI侧边面板：对话窗口，自然语言生成/重构/查bug/注释/单元测试；支持FIM行内代码补全
4. 模型设置面板：API地址、APIKEY、模型选择、超时、代理配置；7 个供应商下拉一键切换（含火山方舟三通道），切换供应商时 Base URL 与默认模型自动填充
5. AI Agent智能体：
   - 收到需求先输出【修改规划】，列出待修改文件清单，等待用户确认
   - 确认后输出diff格式代码变更
   - 支持读取本地选定上下文片段（必须用户确认上传）
6. 终端执行：仅白名单命令`git / cargo / npm / pip`；高危命令拦截，执行前弹窗确认
7. 推送 `v*` 标签触发 GitHub Actions 云端打包并自动发布 Release

### 云端CI流水线模块（GitHub Actions · windows-latest）
1. 免费云端 Windows 构建环境，自动准备 Node(lts)/Rust(stable)
2. 推送 `v*` 标签触发，自动执行 `npm ci` + `npm run tauri build`，输出 NSIS 安装包
3. tauri-action 自动创建 Release、上传安装包；Actions 页保留完整构建日志
4. 不把任何用户 API Key 带进构建，密钥只保存在用户本机

## 目录结构
```

tauri-codex-ide/
├── .github/
│   └── workflows/
│       └── release-windows.yml   # GitHub Actions：v* 标签触发 Windows 云端构建+发 Release
├── docs/
│   └── 开发与部署手册.md          # 环境/推送/发版 SOP/踩坑全记录
├── src-tauri/                 # Rust 后端
│   ├── Cargo.toml
│   ├── tauri.conf.json        # 版本号/窗口/NSIS(currentUser) 配置
│   └── src/
│       ├── main.rs            # 程序入口
│       ├── api_gateway.rs     # API 转发层（7 供应商切换 + FIM 模拟 + 思考标签剥离）
│       ├── file_manager.rs    # 本地文件树、读写、沙箱限制
│       ├── agent.rs           # IDE 内置 Agent 逻辑
│       └── settings.rs        # 本地配置持久化（API 密钥、模型参数）
├── src/                       # Svelte 前端
│   ├── App.svelte
│   ├── main.js
│   ├── components/
│   │   ├── FileTree.svelte
│   │   ├── Editor.svelte      # 精简 Monaco 编辑器
│   │   ├── AiPanel.svelte     # AI 对话侧边栏
│   │   └── Settings.svelte    # 供应商/模型注册表与设置界面
│   └── lib/
│       ├── monaco_setup.js    # 精简 Monaco（本地 worker，无 CDN）
│       └── tauri.js           # invoke 桥接 + 浏览器调试 mock
├── package.json
├── .cursorrules               # Cursor 项目全局规则
├── agent-system-prompt.txt    # IDE 内置 Agent 系统提示词
└── .gitignore

```

## 内置Agent系统提示词（agent-system-prompt.txt）
```

你是轻量化 AI 代码 IDE 内置智能助手，对标 Codex，算力由国内云端 API 提供。
基础规则：

1. IDE 仅在你得到用户确认后，才把选中代码 / 指定片段上传；不会自动上传整个项目源码。
2. 用户给出需求，先输出【修改规划】，列出待修改文件清单，等待用户确认；确认后输出 diff 格式代码变更。
3. 能力：代码生成、重构、Debug、注释、单元测试、FIM 行内补全、源码解释。
4. 执行终端指令前向用户确认；仅允许白名单命令 git /cargo/npm /pip；高危系统命令直接拒绝。
5. 代码块附带完整文件路径；精简输出，减少 token 消耗。
6. 中文交互为主，不编造不存在库、接口。
7. 拒绝生成木马、渗透、恶意脚本。

工作流程：
用户需求 → IDE 读取本地选定上下文片段（用户确认上传）→ 云端思考 → 返回修改方案 → 用户确认 → IDE 本地写入文件 → 执行允许终端命令 → 返回运行结果。

```

## .cursorrules 项目规则
```

项目：Tauri2 AI 代码 IDE，对标 Codex
架构：本地轻量 Tauri 客户端；AI 推理请求转发国内云端 API（火山方舟三通道/MiniMax/DeepSeek），可降级 Ollama 本地；编译打包交给 GitHub Actions 的 windows-latest 云端（v* 标签触发），本机仅写代码。
技术栈强制：Tauri2 + Rust + Svelte + 精简 Monaco Editor，禁止 Electron、React 全家桶。
性能约束：适配 N95 笔记本，客户端空闲内存控制 < 600MB；禁用不必要动画、轮询。
安全规则：

1. 绝不自动上传完整本地项目源码；仅用户手动确认的代码片段才发送到 API。
2. API 密钥保存在本地配置文件，禁止提交密钥到仓库。
3. 文件 IO 做沙箱，只允许在选定工作目录读写；终端命令白名单：git, cargo, npm, pip。高危命令拦截。
4. 所有静态资源本地打包，禁止引用海外 CDN。
代码输出规范：

- 给出完整文件路径，输出完整代码或 diff 片段。
- Rust 后端处理 IO、API 请求、权限校验；前端仅做渲染。
- 优先 MVP 最小功能，非必要功能延后迭代。
- 每次代码变更附带简单自测步骤。

```

## 云端构建与发布（GitHub Actions）

> 完整操作步骤、状态查询、国内下载镜像与踩坑记录见 [docs/开发与部署手册.md](docs/开发与部署手册.md) 第 4、5、7 章。

流水线文件 `.github/workflows/release-windows.yml` 要点：

- 触发条件：推送 `v*` 标签（如 `v0.1.0`），或 Actions 页手动运行；**推 main 不触发**
- Runner：`windows-latest`（公开仓库免费），预装/自动准备 Node lts、Rust stable，带 npm 与 cargo 缓存
- 构建：`npm ci` 后由 `tauri-apps/tauri-action@v0` 执行 Tauri 打包
- 产物：自动创建 GitHub Release 并上传 NSIS 安装包 `TauriCodexIDE_<版本>_x64-setup.exe`
- Tauri CLI 来自 `devDependencies` 的 `@tauri-apps/cli`，无需 `cargo install`；构建会先跑 `beforeBuildCommand`（`npm run build`）

发版三条命令（版本号先在 `package.json` 与 `src-tauri/tauri.conf.json` 改一致）：

```powershell
git push github main                      # 同步代码，不触发构建
git -c user.name='guokaichong' -c user.email='2214866337@qq.com' tag -a v0.2.0 -m 'TauriCodexIDE v0.2.0'
git push github v0.2.0                    # 推标签 → 自动构建并发 Release
```

下载（直连不通时加 `https://ghfast.top/` 等镜像前缀，详见手册）：

```text
https://github.com/guokaichong/tauri-codex-ide/releases
```

> 历史上曾尝试 Gitee Go / Gitee Actions：前者 YAML 报「配置结构错误」且免费云端只有 Linux 构建机（无法打 Tauri Windows 包），后者属企业付费功能，相关文件已于 v0.1.0 之后删除，Gitee 仅保留代码镜像。


## 开发顺序（MVP）

1. 初始化项目，安装依赖，本地 `npm run dev` 调试前端
2. 编写 `file_manager.rs`：文件树、读写文件 + 工作目录沙箱（优先）
3. 编写 `settings.rs`：保存 API 地址、API Key、模型名称（存在本地 json）
4. 编写 `api_gateway.rs`：对接 OpenAI 兼容接口（DeepSeek / 火山 Ark/Ollama 切换）
5. 前端组件：FileTree → Editor → AiPanel → Settings
6. 开发 `agent.rs`：接收需求、生成修改计划、输出 diff
7. 提交代码推 GitHub，打 `v*` 标签触发 GitHub Actions，云端编译打包 Windows 安装包

## 安全风险评估

1. 密钥安全：API 密钥存储在本地 json，严禁提交代码仓库；CI 环境变量只用于构建，不读取用户 API 密钥。
2. 代码上传边界：**不会自动上传整个项目**，必须用户手动确认才发送选中片段到云端 API。
3. N95 性能瓶颈：本地仅 GUI 和 IO 运算；大模型推理上云，降低本地 CPU / 内存压力。
4. 文件沙箱：限制文件读写范围在选定工作目录，防止越权访问系统文件。

## 禁止项

❌ 不要接入 OpenAI 官方接口、[chatgpt.com](https://chatgpt.com)、海外 CDN 资源；
❌ 不要 Electron（体积大、内存高，只用 Tauri2）；
❌ 不要在线云端文件上传，默认项目文件全部本地留存；
❌ 不要自动静默上传代码到外网，用户手动选择才调用云端 API；
❌ 禁止生成木马、渗透、恶意脚本。
