# chronox

[English](README.md) | 简体中文

chronox 是一个用 Rust 编写的命令行工具，用于转换 Unix 时间戳和日期字符串。
它可自动识别输入格式，支持纳秒精度。

## 安装

安装当前稳定版 Rust 工具链后，在仓库根目录运行：

```sh
cargo install --path . --locked
```

## 用法

```sh
chronox now
chronox 1704164645
chronox "2024-01-02T03:04:05Z" --to milliseconds
chronox -a 1704164645123456789
echo 1704164645 | chronox -z UTC
```

默认情况下，时间戳转换为系统时区的可读日期时间，日期字符串转换为以秒为单位的
整数 Unix 时间戳。包含空格的输入需要加引号。未提供输入参数时，chronox 从标准输入
读取一个值，并忽略首尾空白。

`chronox now` 显示当前的秒级 Unix 时间戳和系统时区的可读日期时间，两者对应同一时刻。
使用 `--timezone` 更改显示时区，`--to` 选择一种输出格式，或 `--all` 显示全部格式。
`--unit` 仅适用于数字输入，不能与 `now` 一起使用。

| 选项 | 说明 |
| --- | --- |
| `-t, --to FORMAT` | 选择输出格式，可用值见下表。 |
| `-a, --all` | 显示全部八种输出格式，不能与 `--to` 同时使用。 |
| `-u, --unit UNIT` | 指定数字输入的单位：`s`、`ms`、`us`、`ns` 或对应的英文全名。 |
| `-z, --timezone ZONE` | 指定输出时区；可重复使用以同时显示多个时区。第一个时区也用于解析无时区的日期。 |
| `--list-timezones` | 列出所有可用的 IANA 时区名称并退出。 |
| `-h, --help` | 显示帮助。 |
| `-v, --version` | 显示版本。 |

输出包含识别出的输入格式和显示时区。颜色显示遵循终端设置及 `NO_COLOR` 环境变量；
重定向时输出纯文本。输入无效时，错误信息写入标准错误，退出码为 2。

## 支持的格式

### 输入

| 格式 | 示例 |
| --- | --- |
| Unix 时间戳 | `1704164645`、`1704164645.123456789` |
| 十六进制或二进制时间戳 | `0x65937D25`、`0b1100101100100110111110100100101` |
| ISO 日期时间 | `2024-01-02T03:04:05Z`、`2024-01-02T11:04:05+08:00` |
| ISO 基本格式日期时间 | `20240102T030405Z` |
| ISO 年积日或周日期时间 | `2024-002T03:04:05Z`、`2024-W01-2T03:04:05Z` |
| 可读日期时间 | `2024-01-02 03:04:05.123456`、`2024/01/02 03:04:05` |
| HTTP 日期 | `Tue, 02 Jan 2024 03:04:05 GMT` |
| 邮件日期 | `Tue, 02 Jan 2024 11:04:05 +0800` |
| 仅日期 | `2024-01-02`、`2024/01/02`、`2024-002`、`2024-W01-2` |

时间戳支持秒、毫秒、微秒和纳秒，识别规则见[单位与精度](#单位与精度)。
ISO 日历日期时间和以连字符分隔日期的可读日期时间也支持省略秒，仅保留时和分。
仅日期的输入按相应时区的午夜解析。

HTTP 输入也支持旧版 RFC 850 和 asctime 格式。HTTP 和邮件输入只接受日期值，
不应包含 `Date:` 字段名。

### 输出

| 格式 | `--to` 值 | 别名 |
| --- | --- | --- |
| Unix 秒时间戳 | `seconds` | `s` |
| Unix 毫秒时间戳 | `milliseconds` | `ms` |
| Unix 微秒时间戳 | `microseconds` | `us` |
| Unix 纳秒时间戳 | `nanoseconds` | `ns` |
| 可读日期时间 | `readable` | — |
| ISO 日期时间 | `iso8601` | `rfc3339` |
| HTTP 日期 | `http` | — |
| 邮件日期 | `email` | `rfc2822` |

可读格式使用 `YYYY-MM-DD HH:mm:ss +HH:mm`，有小数秒时会保留。
ISO 输出使用 RFC 3339 格式。可读、ISO 和邮件输出均包含 UTC 偏移量；
ISO 使用 `Z` 表示 UTC。HTTP 输出始终使用 GMT。

## 时区

输入可包含数字偏移量、`Z`、标准 HTTP 或邮件时区字段，
也可在末尾附加 `UTC`、`GMT` 或 `Asia/Singapore` 等 IANA 时区名称。

使用 `-z, --timezone` 可指定 IANA 时区名称、`UTC`（也接受 `GMT` 或 `Z`）、
`local`，或 `+08:00`、`-05:00`、`+0545` 等固定偏移量。重复该选项可同时以多个
时区显示同一时刻；第一个时区也用于解析无时区的日期。`--list-timezones`
会列出所有可用的 IANA 时区名称。

```sh
chronox "2024-01-02 11:04:05 Asia/Singapore" -z UTC --to iso8601
chronox 1704164645 -z=-05:00
chronox 1704164645 -z UTC -z Asia/Singapore
```

时区选择遵循以下规则：

- 输入明确包含时区时，该时区决定实际时刻。`--timezone` 只改变该时刻的显示方式。
- 输入日期不含时区时，使用 `--timezone` 指定的时区；未指定则使用系统时区。
- 未指定 `--timezone` 时，输出使用输入中明确指定的时区；输入无时区则使用系统时区。
- Unix 时间戳与时区无关。各输出格式在各自精度范围内表示同一时刻。

命名时区按输入日期应用夏令时规则，固定偏移量不随日期变化。
本地时间有歧义或不存在时，输入会被拒绝；可使用明确的偏移量指定实际时刻。
输入同时包含偏移量和 IANA 时区名称时，两者必须一致。
`CST` 等地区缩写仅在邮件日期语法有明确定义时接受。

## 单位与精度

整数时间戳根据位数推断单位，不计正负号和前导零：

| 位数 | 单位 |
| --- | --- |
| 1–10 | 秒 |
| 11–13 | 毫秒 |
| 14–16 | 微秒 |
| 17 位及以上 | 纳秒 |

零和小数输入默认以秒为单位。十六进制（`0x`）和二进制（`0b`）输入同样默认以秒
为单位，并遵循 `--unit`。位数无法准确表达所需单位时，使用 `--unit` 指定：
`chronox --unit ms 1000` 表示 Unix 纪元后一秒。负数输入可用 `--` 与选项分隔：
`chronox --unit ns -- -1`。纯数字始终按时间戳处理；仅日期的输入需要使用分隔符。

计算使用整数并保留纳秒精度。整数时间戳转换为较粗的单位时，向负无穷取整。
HTTP 和邮件输出省略小数秒。可读、ISO 和纳秒输出在格式可用时保留完整精度。

支持的 UTC 年份范围为 0001–9999。邮件输出要求显示时区中的年份为 1900–9999。
包含秒的历史时区偏移量无法用可读、ISO 或邮件格式表示，这些格式会标记为不可用，
HTTP 和 Unix 输出仍然可用。不接受闰秒、超出纳秒的精度，
以及 `01/02/2024` 等有歧义的日期格式。

## 开发

使用 `cargo run -- <arguments>` 在本地运行。检查命令：

```sh
cargo test --locked --all-targets
cargo clippy --locked --all-targets -- -D warnings
cargo fmt --all -- --check
```

CI 在 Linux、Windows 和 macOS（Apple Silicon 与 Intel）上运行上述检查及文档测试。
Dependabot 每周检查 Cargo 依赖与 GitHub Actions 更新，分别合并 Rust 的次要版本／
补丁更新和 Actions 更新为各自的 PR。

### 测试覆盖率

CI 使用 [cargo-llvm-cov](https://github.com/taiki-e/cargo-llvm-cov) 生成覆盖率报告。
每次成功的覆盖率任务都会显示摘要，并提供包含 LCOV 和 HTML 报告的 `coverage`
构建产物，保留 14 天。下载并解压后，打开 `html/index.html` 查看详情。
无需外部服务或令牌。在本地生成 HTML 报告：

```sh
rustup component add llvm-tools-preview
cargo install cargo-llvm-cov --locked
cargo llvm-cov --locked --all-targets --html
```

打开 `target/llvm-cov/html/index.html`。当前仅报告覆盖率，不设置最低百分比要求。

### 发布

更新 `Cargo.toml` 中的版本，运行 `cargo check` 刷新 `Cargo.lock`，提交这两个文件，
然后推送匹配的标签，例如：

```sh
git tag v0.1.0
git push origin v0.1.0
```

发布工作流会验证标签与 crate 版本一致，并在 CI 通过后构建二进制文件。
生成的 GitHub Release 包含自动生成的发布说明、SHA-256 校验和（`SHA256SUMS`），
以及包含可执行文件、许可证和 README 的压缩包：

| 平台 | 目标 | 压缩格式 |
| --- | --- | --- |
| Linux x86-64（glibc，在 Ubuntu 24.04 上构建） | `x86_64-unknown-linux-gnu` | `.tar.gz` |
| Windows x86-64 | `x86_64-pc-windows-msvc` | `.zip` |
| macOS Apple Silicon | `aarch64-apple-darwin` | `.tar.gz` |
| macOS Intel | `x86_64-apple-darwin` | `.tar.gz` |

标签必须采用 `vMAJOR.MINOR.PATCH` 格式，可添加 `-beta`、`-beta.N`、`-rc` 或
`-rc.N` 后缀。预发布版本不会替换最新的稳定版本。压缩包附有构建来源证明。
GitHub Release 和构建来源证明使用内置的 `GITHUB_TOKEN`。稳定的 `1.x.y` 版本在
GitHub Release 发布成功后还会发布到 crates.io，需要配置仓库密钥
`CARGO_REGISTRY_TOKEN`。其他版本仅发布到 GitHub Releases。

### 源码结构

解析与格式化共用一个 UTC 时刻，因此输入格式和输出格式可以独立扩展。

| 文件 | 职责 |
| --- | --- |
| [src/parse.rs](src/parse.rs) | 输入识别、单位检测和校验。 |
| [src/output.rs](src/output.rs) | 输出格式、默认格式选择和渲染。 |
| [src/timezone.rs](src/timezone.rs) | 时区解析和本地时间校验。 |
| [src/main.rs](src/main.rs) | 命令行参数、标准输入和退出码。 |
| [src/style.rs](src/style.rs) | 终端颜色和帮助样式。 |
| [src/lib.rs](src/lib.rs) | 公开的解析与格式化 API。 |
