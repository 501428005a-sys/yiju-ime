# AGENTS.md

> 编码约定、架构约束、常用命令沿用上游，见 [CLAUDE.md](CLAUDE.md) 与 [docs/contributing.md](docs/contributing.md)。
> 本文件说明**译句相对青简改了什么、为什么这样改、接下来要做什么**。动手前先读完，别重新摸索。

## 1. 项目是什么

- 译句（YiJu）派生自青简 Qingjian v0.1.4（GPL-3.0-or-later），卖点是**整句翻译**：一句话打完，整句译文自动出现在候选窗下方。
- 目前只维护 **Windows 11**（GNU 工具链编的安装包）。macOS / Linux 的代码原样保留，没有验证过。
- 品牌：产品名「译句」、logo 在 `assets/icon/`（`make-logo.ps1` 生成 PNG 与 ICO）。「译句」名称与 logo 不在 GPL 授权范围内（见 README、NOTICE）。
- **数据来源署名不能改**：词库、释义表、码表、语言模型是青简整理的，`assets/`、`tools/`、「关于」页里写「青简」的出处说明要保留，别批量替换成「译句」。

## 2. 整句翻译（句末译文）

行为：
- 触发：上屏后停顿 1 秒没按键（`PAUSE`）；或上屏文字以 `。？！…!?.` 结尾；或没在组句时按回车。
- 触发即占一行「翻译中…」，译文回来原地替换；每段一行、按打字顺序、各停 `[general] sentence_translation_seconds` 秒（缺省 20，「设置 → 云服务」可改，`RouterConfig::echo_show_for`），最多 4 行（`MAX_LINES`），10 秒没回就收掉（`WAIT_FOR`）。
- 继续打字时译文行接在候选下方，不参与高亮与选词，不上屏。私密输入框、少于 2 个汉字（`MIN_HAN`）不翻译。
- 目标语言 = 学习语言（没开学习语言时英文）。

实现（全部在 Windows Server，DLL / 协议 / Core / 其他平台都没动）：

| 文件 | 作用 |
| --- | --- |
| `apps/windows/server/src/dispatch/echo/mod.rs` | 全部逻辑：攒上屏文字、三种触发、专用 `CloudPredictor` 发 `PredictionKind::Translate`、占位 / 替换 / 过期。常数在文件顶部 |
| `dispatch/message.rs` | 按键后 `note_echo_key` + `tick_echo`；`Poll` 里 `tick_echo`；`SyncMode` 里 `refresh_echo` |
| `dispatch/candidates/{mod,sink}.rs` | `reconcile_candidates` 带上译文行（帧空但有译文时不收窗，位置退到最近的光标矩形）；`CandidateSink::show_with_echo` |
| `ui/mod.rs`、`ui/command.rs`、`ui/candidates/{mod,render_data,row}.rs` | 译文行画成候选后面的行，序号位写「译」（`row::echo`） |
| `dispatch/key/`、`dispatch/session/`、`dispatch/reload/`、`main.rs` | 导航键判定、换会话清半句、启动 / 热加载时 `attach_echo` |

为什么这样设计：
- **不改 `Frame` / 协议**：`Frame` 在 Linux、macOS 也有构造，老 DLL 也要能解析。译文行只进 Server 自绘窗；**发给 DLL 的帧里绝不能有它**（DLL 见到非空帧会以为在组句、开始吃键）。
- **专用云端通道**：组句联想每敲一键换序号，Core 会把旧结果当过期丢掉；走 `Engine::request_translation` 的话接着打字译文就被冲掉。
- **节拍**：Server 没有定时器。DLL 组句时每 80 ms 发 `Poll`，没在组句且前台时每 320 ms 发 `SyncMode`，停顿判定与过期都借这两拍。
- 青简渲染器的提示行（`notice` / `status`）只有一行，还会顶掉拼音行右侧的整句补全，所以译文画成候选下方的行。
- DLL 组句时只转发 `Ctrl+1~9`（`apps/windows/tsf/src/com/service/key_sink.rs` 的 `eats_key`），加新快捷键要连 DLL 一起改。
- 上游约定「平台层不许调翻译」（contributing.md）。译句为了不动 Core 暂时放在 Server；要回馈上游时得挪进 Core 并加默认关闭的开关。

## 3. 编译与验证

```powershell
$env:QINGJIAN_UIACCESS = '0'   # 没有签名证书时必须
cargo build --release -p qingjian-windows-server
cargo fmt -p qingjian-windows-server
cargo clippy --release -p qingjian-windows-server --all-targets
cargo test  --release -p qingjian-windows-server
```

- MSVC（上游方式）或 GNU 工具链（`x86_64-pc-windows-gnu` + MinGW-w64）都能编 Server。GNU 产物只依赖系统 DLL。
- 试用：把 `target\release\qingjian-server.exe` 替换进已安装的输入法目录（先备份），重启 Server。
- 日志：`%LOCALAPPDATA%\Qingjian\logs\server.<UTC 日期>.log`。每次启动应看到两条「云联想已启用」（第二条是整句翻译的专用通道）。

## 4. 待办

1. **安装包（0.1.0 已能打）**：`apps\windows\installer\build.ps1 -Gnu`（GNU 工具链）或不带 `-Gnu`（MSVC，上游方式）。
   - 装到 `{autopf}\YiJu`，新 AppId；**与青简互斥**：安装时检测到青简就提示先卸载（`[Code] RemoveQingjian`），两者共用 TSF 的
     CLSID / profile GUID、命名管道 `qingjian`、数据目录 `%APPDATA%\Qingjian`（所以青简用户的配置与学习数据直接沿用）。
     要做成可并存，得换掉这些内部标识（CLSID / profile / 显示属性 GUID 在 `apps/windows/tsf/src/com/`，管道名在
     `crates/qingjian-platform/src/protocol/codec.rs`，互斥体 `*Qingjian*`，窗口类名 `Qingjian*`，数据目录在 `crates/qingjian-platform/src/dirs.rs`）。
   - `-Gnu` 的不同：32 位 DLL 用 `i686-pc-windows-gnu`；设置程序的 Windows App Runtime 由 build.ps1 自己从 NuGet 取（缓存
     `target\gnu-runtime`），自包含清单 `apps/windows/settings/self-contained.manifest` 由 build.rs 经 winresource 嵌入；
     产物会 strip；**GNU 链接器没有 `/DELAYLOAD`，设置程序只能在 Windows 11 上启动**，所以 `-Gnu` 包的 MinVersion 是 10.0.22000。
     要支持 Windows 10：用 MSVC 编设置程序，或给 GNU 版做延迟加载导入。
   - 安装包没有代码签名（SmartScreen 会拦）；Server 不带 uiAccess（候选窗在开始菜单 / 任务栏搜索里可能被盖住）。
2. **检查更新**：目前缺省关闭，`INDEX_URL` 指向本仓库 Releases 的 `releases.json`，但签名公钥还是上游的（`crates/qingjian-update`）。要启用得换成自己的 ed25519 密钥并在发版时签名。
3. **打字准确度**：青简 0.1 的词库 / 语言模型偏小。派生项目 `rambocode/glimmer`（微明）0.1.10 换了全量语料三元语言模型、补了口语词，可考虑移植其数据。缺省领域词库只开了成语，可以考虑缺省全开。
4. macOS：`assets/icon/menu.pdf` 已删，打包前要从 `menu.svg` 重新导出（命令见 `assets/icon/README.md`）。
5. 用户文档 `docs/user/` 还没写整句翻译的说明页。
