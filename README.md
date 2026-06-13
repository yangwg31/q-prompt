# Q-Prompt

桌面悬浮提示词工具栏 — 点击即可将预置提示词插入任意编辑器。

## 特性

- **毛玻璃悬浮条** — 半透明置顶，hover 变实，可拖拽移动
- **一键插入** — 点击标签或 `Alt+1~8` 直接将提示词插入光标位置
- **多选拼接** — `Shift+点击` 多选，用 `---` 分割后一并插入
- **变量占位符** — 支持 `{{变量名}}` 和 `{{selection}}`（自动获取选中文本）
- **管理面板** — 新增/编辑/删除/拖拽排序/搜索/导入导出 JSON
- **快捷键可配置** — 所有快捷键可在管理面板中修改
- **8 条预设模板** — 首次启动自动注入
- **系统托盘** — 右键菜单控制显示模式（始终显示/IDE智能/隐藏）

## 安装

1. 从 [Releases](https://github.com/yangwg31/q-prompt/releases) 下载最新 `.msi` 安装包
2. 双击安装，完成后自动启动
3. 按 `Alt+Q` 显示/隐藏悬浮条

## 开发环境搭建

### 前置依赖

- [Rust](https://rustup.rs) >= 1.77
- [Node.js](https://nodejs.org) >= 18
- [Visual Studio Build Tools](https://visualstudio.microsoft.com/downloads/#build-tools-for-visual-studio-2022)（勾选 "Desktop development with C++"）

### 安装与运行

```bash
# 安装依赖
npm install

# 开发模式（热重载）
npm run tauri dev

# 生产构建
npm run tauri build
```

## 快捷键

| 快捷键 | 功能 |
|--------|------|
| `Alt+Q` | 切换悬浮条显示/隐藏 |
| `Alt+1` ~ `Alt+8` | 插入对应位置提示词 |
| `Alt+S` | 将选中文本保存为新提示词 |
| `Esc` | 关闭管理面板 |

所有快捷键可在管理面板的「快捷键」Tab 中自定义修改。

## 数据存储

所有数据仅存储在本地：

```
%APPDATA%/q-prompt/
├── config.json          # 偏好设置 + 快捷键
├── prompts.json         # 提示词数据
├── deleted_backup.json  # 已删除提示词的备份
└── q-prompt.log         # 运行日志
```

## 技术栈

- **桌面框架**: Tauri v2 (Rust)
- **前端**: 纯 HTML/CSS/JS，零框架依赖
- **毛玻璃效果**: CSS `backdrop-filter: blur`
- **全局快捷键**: `tauri-plugin-global-shortcut`
- **剪贴板桥接**: `arboard` + `enigo` 模拟 Ctrl+V

## License

MIT
