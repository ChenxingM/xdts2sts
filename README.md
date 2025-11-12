# XDTS2STS

XDTS/TDTS 转换 STS 摄影表转换工具

## 📋 系统要求

### 运行要求
- Windows 7 或更高版本
- 无需安装任何依赖

### 编译要求
- Rust 1.70 或更高版本
- 安装 Rust: https://rustup.rs/

## 🚀 快速开始

### 方式一：使用预编译版本
1. 下载 `xdts2sts.exe`
2. 拖放文件或文件夹到 exe 上

### 方式二：从源码编译
```bash
# 克隆或下载项目
cd rust-xdts2sts

# 编译
cargo build --release
```

## 📖 使用方法

### 1. 转换单个文件
- 拖放 `.xdts` 或 `.tdts` 文件到 `xdts2sts-rust.exe`
- 生成的 `.sts` 文件会保存在原文件同目录

### 2. 批量转换文件夹
- 拖放包含 xdts/tdts 文件的文件夹到 `xdts2sts-rust.exe`
- 程序会自动查找所有 `.xdts` 和 `.tdts` 文件
- 生成的 `.sts` 文件会保存在 exe 同目录的 `converted_sts` 文件夹中

### 3. 多文件/文件夹
- 可以同时拖放多个文件或文件夹
- 程序会显示进度并逐个处理

## 🏗️ 项目结构

```
rust-xdts2sts/
├── Cargo.toml          # 项目配置
├── README.md           # 说明文档
└── src/
    ├── main.rs         # 主程序入口
    ├── types.rs        # 数据类型定义
    ├── parser.rs       # XDTS/TDTS 解析器
    └── converter.rs    # STS 转换器
```


## 🔧 技术细节

### 依赖库
- `serde` + `serde_json`: JSON 解析
- `encoding_rs`: Shift-JIS 编码支持
- `walkdir`: 文件夹遍历
- `anyhow`: 错误处理

### 支持的格式

#### 输入格式
- **XDTS**
- **TDTS**

#### 输出格式
-  `*.sts`: ShiraheiTimeSheet 二进制格式

## 🤝 贡献

欢迎提交 Issue 和 Pull Request！

## 📜 许可证

MIT License
