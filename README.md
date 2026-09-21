# TauriCodexIDE — 轻量化AI代码IDE（对标Codex，国内环境）
> 项目定位：本地轻量GUI客户端，AI推理使用国内云端API，编译打包全部放到Gitee云端CI流水线；适配 N95 / 12~16G Windows10 LTSC2021 低配笔记本，绿色免安装Windows单EXE产物。

## 核心架构思路
- **本地端**：Tauri2 + Rust + Svelte + 精简Monaco Editor
  只做GUI、本地文件读写、终端白名单命令执行、API请求转发。**本地不跑大模型推理**
- **AI算力**：优先国内云端API（火山引擎Ark / DeepSeek API，兼容OpenAI协议）；可一键降级本地Ollama(127.0.0.1:11434)
- **编译打包**：Rust编译、Tauri打包、资源打包全部在Gitee云端CI流水线执行（见下方「Gitee CI 配置」章节），本机仅写代码。
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

## 功能清单
### 本地IDE客户端模块
1. 项目工作区 & 文件树：本地文件夹导入、多文件浏览读写；**沙箱限制，禁止跨目录读取系统敏感文件**
2. 精简代码编辑器：Monaco Editor裁剪版，语法高亮、代码选中唤起AI对话
3. AI侧边面板：对话窗口，自然语言生成/重构/查bug/注释/单元测试；支持FIM行内代码补全
4. 模型设置面板：API地址、APIKEY、模型选择、超时、代理配置；云端API / Ollama本地一键切换
5. AI Agent智能体：
   - 收到需求先输出【修改规划】，列出待修改文件清单，等待用户确认
   - 确认后输出diff格式代码变更
   - 支持读取本地选定上下文片段（必须用户确认上传）
6. 终端执行：仅白名单命令`git / cargo / npm / pip`；高危命令拦截，执行前弹窗确认
7. 一键推送源码触发Gitee CI云端打包

### 云端CI流水线模块（Gitee Go）
1. 云端Windows构建环境，预装Node/Rust
2. 自动执行 `npm run tauri build`，输出绿色免安装单EXE
3. 制品归档，生成下载链接；保留构建日志，失败返回完整报错
4. 使用Gitee环境变量存放密钥，不硬编码到代码

## 目录结构
```

tauri-codex-ide/
├── .gitee/
│   └── workflows/
│       └── build.yml          # GitHub-Actions 兼容语法（Gitee 是否识别待验证）
├── src-tauri/                 # Rust 后端
│   ├── Cargo.toml
│   ├── tauri.conf.json
│   └── src/
│       ├── main.rs            # 程序入口
│       ├── api_gateway.rs     # API 转发层（DeepSeek / 火山 Ark/Ollama 切换）
│       ├── file_manager.rs    # 本地文件树、读写、沙箱限制
│       ├── agent.rs           # IDE 内置 Agent 逻辑
│       └── settings.rs         # 本地配置持久化（API 密钥、模型参数）
├── src/                       # Svelte 前端
│   ├── App.svelte
│   ├── main.js
│   ├── components/
│   │   ├── FileTree.svelte
│   │   ├── Editor.svelte      # 精简 Monaco 编辑器
│   │   ├── AiPanel.svelte     # AI 对话侧边栏
│   │   └── Settings.svelte
│   └── lib/
│       ├── monaco_setup.js    # 精简 Monaco（本地 worker，无 CDN）
│       └── tauri.js           # invoke 桥接 + 浏览器调试 mock
├── .workflow/
│   └── build.yml              # Gitee Go 原生语法（Gitee 官方目录，正式生效）
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
架构：本地轻量 Tauri 客户端；AI 推理请求转发国内云端 API（火山 Ark/DeepSeek），可降级 Ollama 本地；编译打包交给 Gitee CI，本机仅写代码。
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

## Gitee CI 配置

仓库内保留两份流水线定义，语法不同、互不覆盖：

| 文件 | 语法 | 说明 |
| --- | --- | --- |
| `.workflow/build.yml` | Gitee Go 原生语法 | Gitee 官方文档确认的流水线目录，正式生效 |
| `.gitee/workflows/build.yml` | GitHub-Actions 兼容语法 | 与本文档原示例对应；Gitee 是否识别该目录，需推到远端后在仓库「流水线」页签确认 |

推送后确认哪一份产生了构建记录，再删除另一份即可。

### 1. Gitee Go 流水线（.workflow/build.yml）

> 原示例中的 `shturl.cc` 短链 action 非法，此文件已改为 Gitee Go 原生 `shell@windows` 步骤，不依赖任何外部 action。

```yaml
name: tauri-windows-build
displayName: TauriCodexIDE Windows 打包
triggers:
  push:
    branches:
      include: [master, main]
stages:
  - stage:
      name: build
      jobs:
        - job:
            name: windows-build
            runsOn: x86_64_windows   # 需 Gitee Go Windows 资源池或自托管 Windows 节点
            steps:
              - step: shell@windows
                displayName: 安装依赖
                inputs:
                  commands:
                    - npm config set registry https://registry.npmmirror.com
                    - npm install --no-audit --no-fund
              - step: shell@windows
                displayName: 云端打包
                inputs:
                  commands:
                    - npm run tauri build
              - step: artifacts@agent
                displayName: 归档安装包
                inputs:
                  name: tauri-codex-ide-windows
                  paths:
                    - src-tauri/target/release/bundle/**
                  retentionDays: 14
```

Tauri CLI 由 `devDependencies` 中的 `@tauri-apps/cli` 提供（版本与 `package.json` 锁定），因此无需 `cargo install tauri-cli`；`npm run tauri build` 会先执行 `beforeBuildCommand`（前端构建）再编译打包。

产物：`src-tauri/target/release/tauri-codex-ide.exe`（可直接运行的单文件）与 `bundle/nsis/*.exe`（安装包）。
注意：NSIS 工具链首次构建需从网络拉取，Windows 资源池需能访问公网或配置缓存。

### 2. GitHub-Actions 兼容流水线（.gitee/workflows/build.yml）

```yaml
name: Build Tauri Windows EXE
on:
  push:
    branches: [ main ]
  workflow_dispatch:

jobs:
  build:
    runs-on: windows-latest
    steps:
      - name: Checkout code
        uses: actions/checkout@v4
      - name: Setup Node.js
        uses: actions/setup-node@v4
        with:
          node-version: 22
          cache: 'npm'
      - name: Setup Rust
        uses: dtolnay/rust-toolchain@stable
        with:
          toolchain: stable
          override: true
      - name: Npm install
        run: npm install
      - name: Build Tauri
        run: npm run tauri build
      - name: Upload artifact
        uses: actions/upload-artifact@v4
        with:
          name: tauri-codex-build
          path: src-tauri/target/release/bundle/
```

> 该文件中的 `actions/*` 是 `owner/repo@tag` 相对写法，runner 默认从 github.com 拉取 action 脚本。若 Gitee 未提供镜像，中国大陆网络下该流水线可能拉取失败 —— 这也是第 1 份 Gitee Go 流水线更稳妥的原因。

## 开发顺序（MVP）

1. 初始化项目，安装依赖，本地 `npm run dev` 调试前端
2. 编写 `file_manager.rs`：文件树、读写文件 + 工作目录沙箱（优先）
3. 编写 `settings.rs`：保存 API 地址、API Key、模型名称（存在本地 json）
4. 编写 `api_gateway.rs`：对接 OpenAI 兼容接口（DeepSeek / 火山 Ark/Ollama 切换）
5. 前端组件：FileTree → Editor → AiPanel → Settings
6. 开发 `agent.rs`：接收需求、生成修改计划、输出 diff
7. 提交代码到 Gitee，开启云端流水线，编译打包 Windows 单 exe

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
