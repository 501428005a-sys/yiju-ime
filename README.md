<p align="center">
  <img src="assets/icon/logo.png" width="128" alt="译句输入法">
</p>

<h1 align="center">译句输入法 YiJu</h1>

<p align="center"><strong>打完一句中文，整句英文翻译就出来了。</strong><br>
Type a sentence in Chinese — get the whole sentence in English, right under your candidates.</p>

<p align="center">
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-GPL--3.0--or--later-blue" alt="GPL-3.0-or-later"></a>
  <img src="https://img.shields.io/badge/Windows-10%2F11-blue" alt="Windows 10/11">
  <img src="https://img.shields.io/badge/free-open%20source-brightgreen" alt="免费开源">
</p>

---

## 为什么做译句

大多数「学英语输入法」只能告诉你**一个词**怎么说。可真正想知道的，往往是**这一整句话**用英文怎么讲。

译句做的就是这件事：照常用拼音打字，**一句话打完，整句的英文翻译自动出现在候选窗口下方**。不用复制、不用切软件、不打断输入。

```text
jintiantianqihenhao␣         ← 照常打字、上屏
                              ← 停顿 1 秒
 1 今天天气很好
 …
 译 The weather is really nice today.
```

## 整句翻译怎么用

- **自动触发**，三种情况任一即可：
  - 上屏后**停顿 1 秒**没再按键；
  - 打出句末标点 `。` `？` `！`；
  - 在聊天框里按**回车**发送。
- 触发后立刻出现一行「**译 翻译中…**」，译文一回来就原地替换。
- **一段一行**：连着打几段，每段的译文各占一行、按打字顺序排列，每行停留 5 秒。
- **不打断输入**：继续打字时，译文行留在候选列表下面，不参与选词、不会被误选，也不会自动上屏。
- 默认译成英文；在设置里把学习语言换成日语 / 西班牙语，就译成对应语言。
- 密码框里不翻译；少于两个汉字的片段不翻译。

整句翻译使用你自己在「设置 → 云服务」里填写的 AI 服务（缺省 DeepSeek，任何 OpenAI 兼容接口都可以）。请求从你的电脑**直接**发往服务商，不经过任何第三方服务器。

## 其他功能

译句派生自开源输入法[青简](https://github.com/qingjian-team/qingjian)，以下能力都完整保留：

- 候选词旁显示单词译文与词性，`Ctrl + 数字` 直接上屏译文；
- 选中任意文字按 `Ctrl + Alt + T` 翻译；
- 整句拼音输入、简拼、拼写纠错、本地整句模型重排；
- 全拼 / 七种双拼 / 五笔 / 注音；模糊音；领域词库；
- 云联想：云端候选词、整句补全（`Tab` 接受）、问字；
- 学习你的用词习惯，全部数据只存在本机。

## 现状

- 当前版本基于青简 0.1.4，**仅 Windows**（10 1809+ / 11，64 位）。
- 独立安装包正在制作中，发布后在 [Releases](https://github.com/501428005a-sys/yiju-ime/releases) 下载。
- 开发者可先体验：装好青简 0.1.4 后，用本仓库编出的 `qingjian-server.exe` 替换安装目录里的同名文件（先备份），注销重新登录即可。

## 配置云服务

1. 打开「设置 → 云服务」，打开云联想开关；
2. 填写服务地址、模型与 API 密钥（DeepSeek 在其开放平台申请密钥）；
3. 点「测试连接」，成功后整句翻译与云联想即可使用。

密钥只保存在本机（`%APPDATA%\Qingjian\.env`），不会写进配置文件导出，也不会出现在日志里。

## 从源码编译（Windows）

官方工具链（MSVC）与上游一致，见 [apps/windows/README.md](apps/windows/README.md)。不想装 Visual Studio 时也可以用 Rust 的 GNU 工具链：

```powershell
rustup toolchain install 1.96.0-x86_64-pc-windows-gnu   # 另需 MinGW-w64（如 winlibs）在 PATH 中
$env:QINGJIAN_UIACCESS = '0'                             # 没有代码签名证书时关掉 uiAccess
cargo build --release -p qingjian-windows-server
cargo test  --release -p qingjian-windows-server
```

整句翻译的实现在 [`apps/windows/server/src/dispatch/echo/`](apps/windows/server/src/dispatch/echo/mod.rs)，设计说明见 [AGENTS.md](AGENTS.md)。

## 来源与致谢

译句派生自 [青简 Qingjian](https://github.com/qingjian-team/qingjian)（GPL-3.0-or-later）。输入引擎、平台壳、数据与工具链均来自该项目，感谢原作者与贡献者。原有版权声明全部保留；本仓库的改动同样以 GPL-3.0-or-later 发布，改动说明见 [NOTICE](NOTICE)。

随包数据（词库、语言模型、释义表、emoji、英文词表、词汇等级、码表）各自遵循来源的许可证，清单见 [docs/design/landscape.md](docs/design/landscape.md)。

## 许可与品牌

- 代码以 **GPL-3.0-or-later** 发布（见 [LICENSE](LICENSE)）：可以自由使用、修改与再分发，**修改后分发须同样以 GPL 开源全部源码**。
- **「译句」名称与 logo 不在授权范围内。** 基于本项目发布的修改版请使用其他名称与图标，并注明来源。
- 译句永久免费。若你为获得它向他人付费，你被骗了。
