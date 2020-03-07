# USTC Get Grade
[![dependency status](https://deps.rs/repo/github/weirane/ustc-get-grade/status.svg)](https://deps.rs/repo/github/weirane/ustc-get-grade)

自动从 USTC 新教务系统中获取成绩并利用邮件通知。

### 运行
Rust 版本要求：1.40+。[安装方法](https://rustup.rs/)

```sh
cp config_example.toml config.toml
$EDITOR config.toml
cargo run --release
```

关于配置文件 `config.toml`：其中的密码可以使用明文 `password = "foo"`，或者使用
一个命令 `pass_exec = "command"`，此命令的 stdout 截去末尾的换行符将作为密码，可
以配合 `gpg` 或其它密码管理器使用。此命令只在加载配置文件时执行一次。

### 做为一个库使用
在 `Cargo.toml` 中加入
```toml
[dependencies.ustc-get-grade]
git = "https://github.com/weirane/ustc-get-grade"
default-features = false
```
