# AGENTS.md - rdrive

## 适用范围

本文件适用于整个仓库。子目录中的 `AGENTS.md` 只对对应目录生效，并覆盖或补充本文件规则。

## 项目结构

- `rdrive/`: 核心动态驱动管理库，覆盖驱动注册、FDT/PCI 探测、设备生命周期、
  `Device<T>` 所有权模型和 `module_driver!` 导出入口。
- `rdrive-macros/`: 面向驱动注册和模块生成的 proc-macro crate。
- `interface/rdif-*`: no_std 驱动接口 crate，定义基础类型、错误、IRQ、串口、定时器、
  块设备、电源、网络、PCIe 等硬件抽象接口。
- `bus/pcie/`: no_std PCIe 总线枚举和控制器支持，包含 `bare-test.toml` 裸机测试配置。
- `examples/enumerate/`: 基于 QEMU FDT 数据的驱动枚举示例。
- `data/`: 示例和测试使用的 QEMU DTS/DTB 数据。
- `.github/workflows/test.yml`: 当前仓库 CI 的构建和测试入口。
- `.github/workflows/release-plz.yml` 与 `release-plz.toml`: release-plz 发布流程来源。

## 依赖与工具

- 优先使用仓库声明的入口和版本：`Cargo.toml` workspace、`rust-toolchain.toml`、
  `.cargo/config.toml`、GitHub Actions workflow 和 package 自带配置。
- 不要为了通过检查临时引入仓库未声明的工具、依赖或配置；新增或替换工具链、
  runner、安装脚本、发布配置等应作为独立变更说明依据。
- 本地 `rust-toolchain.toml` 声明 nightly 且包含 `rust-src`、`rustfmt`、`clippy`；
  当前 CI workflow 安装 stable 并运行 `cargo build` 与 `cargo test -p rdrive`。
  两者不一致时，先如实说明验证面，不要顺手改工具链。
- `.cargo/config.toml` 为 `target_os = "none"` 配置了 `ostool cargo-test` runner。
  只有在修改裸机测试或 PCIe 裸机路径且环境可用时才依赖该 runner。

## Git 与提交

- 除非用户明确要求留在当前分支，否则仓库改动应在功能分支上完成。
- 遵循近期提交风格，使用 Conventional Commits，例如 `feat: ...`、`fix: ...`、
  `chore(rdif-serial): ...`、`refactor: ...`。中文说明可以用于提交摘要或正文。
- 不要把无关改动混入同一个提交。只暂存属于当前任务的文件。
- 提交信息、PR 标题、PR 正文、release note 和上游可见文档只写仓库可见变更、
  reviewer 需要的动机和验证证据；不要写本机路径、临时容器名、私有计划或个人流程。

## 验证

- Rust 改动优先运行覆盖被修改区域的最小检查。当前 CI 基线是：
  `cargo build` 和 `cargo test -p rdrive`。
- 影响多个 workspace crate、公共接口或宏展开行为时，优先扩大到
  `cargo test --workspace`；如果某些 no_std 或裸机 target 受环境限制，说明未运行原因。
- 影响格式或 lint 风险较高的 Rust 改动，尽量运行
  `cargo fmt --all -- --check` 和 `cargo clippy --workspace --all-targets -- -D warnings`；
  如果当前工具链或 target 组合无法完成，应记录失败原因，不要把它误报为 CI 要求。
- 修改 `bus/pcie/`、`interface/rdif-pcie/` 或裸机测试配置时，检查
  `bus/pcie/bare-test.toml` 与 `.cargo/config.toml` 的 runner 假设是否仍成立。

## 文档与发布

- 用户可见的驱动注册、FDT/PCI 探测、设备所有权模型、接口 trait、错误类型、
  示例集成流程发生变化时，同步更新 `README.md` 或对应 package 文档。
- package `CHANGELOG.md` 按 crate 拆分。只有发布、版本变更或用户明确要求时才更新。
- `release-plz.toml` 和 release workflow 属于发布面；非发布任务不要顺手调整版本、
  changelog 或 release-plz 配置。

## Rust 约定

- 保持各 package 已采用的 edition、no_std 边界和公开 API 风格；不要把 `std` 依赖引入
  no_std crate，除非放在 `cfg(test)`、example 或明确的 feature 边界内。
- `rdif-*` trait、错误类型、`rdrive` 公共函数、`Device<T>` 所有权语义和
  `module_driver!` 宏都是下游集成面。修改前确认 semver 影响，并补充示例或测试证据。
- FDT、PCI 配置空间、driver register、probe kind 等结构化数据应使用已有类型和 parser；
  不要手写脆弱的字符串处理。
- `unsafe`、裸指针、link section、MMIO、Send/Sync 手动实现和设备强制访问属于高风险路径。
  新增或重写 unsafe 代码时保持 unsafe block 尽量小，并写清 `// SAFETY:` 不变量。
- 并发和设备借用改动必须维护 `Device<T>` 的独占访问、owner 标识、weak reference 与中断
  handler 快速索引语义；不要把普通 `Mutex` 心智模型直接套到该所有权模型上。
