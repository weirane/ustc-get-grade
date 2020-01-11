# USTC Get Grade
自动从 USTC 新教务系统中获取成绩并利用邮件通知。

### 运行（TODO）
Rust 版本要求：1.39+。[安装方法](https://rustup.rs/)

```sh
cp config_example.toml config.toml
$EDITOR config.toml
cargo run --release
```

### 做为一个库使用
在 `Cargo.toml` 中加入
```toml
[dependencies.ustc-get-grade]
version = "0.0.1"
git = "https://github.com/weirane/ustc-get-grade"
default-features = false
```
